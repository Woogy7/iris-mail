//! Helper for paginating Microsoft Graph list endpoints.
//!
//! Graph list endpoints return a single page of items (default 100) plus an
//! `@odata.nextLink` absolute URL when more results exist. [`paginate`]
//! follows that link transparently and concatenates the per-page `value`
//! arrays into a single `Vec<T>`, capping the total at a caller-supplied
//! `max_items` to bound memory and request volume.
//!
//! # Module split
//!
//! The public-to-this-crate entry point is [`paginate`], which takes a
//! Graph-relative path (e.g. `"/me/mailFolders?$top=100"`) and prepends the
//! Graph base URL. The lower-level [`paginate_from_url`] takes a fully
//! qualified absolute URL and is split out so unit tests can point the first
//! request at a `wiremock::MockServer` instead of `graph.microsoft.com`.
//! Production code should always call [`paginate`].
//!
//! # Error policy
//!
//! Errors are never silently swallowed. If any page (initial or follow-up)
//! fails to fetch or deserialise, the helper returns
//! [`crate::Error::Graph`] with a message that includes the page index so a
//! mid-pagination failure is identifiable. Callers receive no partial
//! results on error.

use serde::de::DeserializeOwned;

use crate::graph::client::{GRAPH_BASE_URL, GraphClient};
use crate::graph::types::GraphResponse;

/// Fetches all items from a paginated Graph list endpoint, up to `max_items`.
///
/// `initial_path` is a Graph-relative path including any query string
/// (e.g. `"/me/mailFolders?$top=100"`). It is appended to the Graph base URL
/// before the first request. Subsequent requests follow the absolute
/// `@odata.nextLink` URL returned by Graph.
///
/// `max_items` is a **total cap across all pages**, not a page size. Page
/// size is determined by the caller's `$top=` query parameter (or Graph's
/// default of 100). When extending the accumulator past the cap, the result
/// is truncated to exactly `max_items` and no further pages are fetched.
///
/// Returns `Err(crate::Error::Graph(...))` on the first fetch or
/// deserialisation failure; partial results are not returned on error.
#[allow(dead_code)] // Wired up by folders.rs / messages.rs in subsequent tasks.
pub(crate) async fn paginate<T>(
    client: &GraphClient,
    initial_path: &str,
    max_items: usize,
) -> crate::Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let url = format!("{GRAPH_BASE_URL}{initial_path}");
    paginate_from_url(client, &url, max_items).await
}

/// Same as [`paginate`] but takes a fully qualified absolute URL for the
/// first request.
///
/// Exposed at `pub(crate)` visibility so tests can target a local
/// `wiremock::MockServer`. Production callers should use [`paginate`].
#[allow(dead_code)] // Used via `paginate` in subsequent tasks; called directly by unit tests today.
pub(crate) async fn paginate_from_url<T>(
    client: &GraphClient,
    initial_url: &str,
    max_items: usize,
) -> crate::Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let mut acc: Vec<T> = Vec::new();
    let mut next: Option<String> = Some(initial_url.to_string());
    let mut page_index: usize = 0;

    while let Some(url) = next.take() {
        if acc.len() >= max_items {
            break;
        }

        let resp = client.get_url(&url).await?;
        let page: GraphResponse<T> = resp.json().await.map_err(|e| {
            crate::Error::Graph(format!("failed to parse Graph page {page_index}: {e}"))
        })?;

        acc.extend(page.value);
        if acc.len() > max_items {
            acc.truncate(max_items);
            break;
        }

        next = page.next_link;
        page_index += 1;
    }

    Ok(acc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Minimal item type used by the pagination tests.
    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Item {
        id: u32,
    }

    fn client() -> GraphClient {
        GraphClient::new("test-token".to_string())
    }

    #[tokio::test]
    async fn single_page_no_next_link_returns_immediately() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": 1}, {"id": 2}, {"id": 3}],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let url = format!("{}/page1", server.uri());
        let items: Vec<Item> = paginate_from_url(&client(), &url, 100).await.unwrap();

        assert_eq!(items, vec![Item { id: 1 }, Item { id: 2 }, Item { id: 3 }]);
    }

    #[tokio::test]
    async fn follows_next_link_across_two_pages() {
        let server = MockServer::start().await;
        let next_url = format!("{}/page2", server.uri());

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": 1}, {"id": 2}],
                "@odata.nextLink": next_url,
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/page2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": 3}, {"id": 4}],
            })))
            .expect(1)
            .mount(&server)
            .await;

        let url = format!("{}/page1", server.uri());
        let items: Vec<Item> = paginate_from_url(&client(), &url, 100).await.unwrap();

        assert_eq!(
            items,
            vec![
                Item { id: 1 },
                Item { id: 2 },
                Item { id: 3 },
                Item { id: 4 },
            ]
        );
    }

    #[tokio::test]
    async fn respects_max_items_cap_mid_page() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [
                    {"id": 1},
                    {"id": 2},
                    {"id": 3},
                    {"id": 4},
                    {"id": 5},
                ],
                "@odata.nextLink": "https://example.invalid/should-not-be-fetched",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let url = format!("{}/page1", server.uri());
        let items: Vec<Item> = paginate_from_url(&client(), &url, 3).await.unwrap();

        assert_eq!(items, vec![Item { id: 1 }, Item { id: 2 }, Item { id: 3 }]);
    }

    #[tokio::test]
    async fn respects_max_items_cap_at_page_boundary() {
        let server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": 1}, {"id": 2}, {"id": 3}],
                "@odata.nextLink": "https://example.invalid/should-not-be-fetched",
            })))
            .expect(1)
            .mount(&server)
            .await;

        let url = format!("{}/page1", server.uri());
        let items: Vec<Item> = paginate_from_url(&client(), &url, 3).await.unwrap();

        assert_eq!(items, vec![Item { id: 1 }, Item { id: 2 }, Item { id: 3 }]);
    }

    #[tokio::test]
    async fn propagates_error_on_second_page() {
        let server = MockServer::start().await;
        let next_url = format!("{}/page2", server.uri());

        Mock::given(method("GET"))
            .and(path("/page1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "value": [{"id": 1}],
                "@odata.nextLink": next_url,
            })))
            .expect(1)
            .mount(&server)
            .await;

        // Page 2 responds 200 OK but with malformed JSON, exercising the
        // per-page parse error path (which carries the page index).
        Mock::given(method("GET"))
            .and(path("/page2"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "application/json")
                    .set_body_string("{ not json"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let url = format!("{}/page1", server.uri());
        let result: crate::Result<Vec<Item>> = paginate_from_url(&client(), &url, 100).await;

        let err = result.expect_err("expected error from page 2");
        let msg = err.to_string();
        assert!(
            msg.contains("page 1"),
            "expected page index 1 in error message, got: {msg}"
        );
    }
}
