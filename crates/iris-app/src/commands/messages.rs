/// List messages in a folder with pagination.
#[tauri::command]
pub async fn list_messages(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    folder_id: String,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<iris_core::Message>, String> {
    let id = iris_core::FolderId(uuid::Uuid::parse_str(&folder_id).map_err(|e| e.to_string())?);
    iris_db::repo::MessageRepo::list_by_folder(&pool, &id, limit.unwrap_or(50), offset.unwrap_or(0))
        .await
        .map_err(|e| e.to_string())
}
