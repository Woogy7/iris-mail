use tauri::Manager;

/// Application configuration available to all Tauri commands.
///
/// Loaded once at startup from environment variables and then shared
/// via Tauri's managed state.
pub struct AppConfig {
    /// M365 OAuth client ID (from `IRIS_M365_CLIENT_ID` env var).
    pub m365_client_id: Option<String>,
    /// Port for the OAuth localhost redirect listener.
    pub oauth_redirect_port: u16,
}

impl AppConfig {
    /// Load configuration from environment variables.
    pub fn from_env() -> Self {
        Self {
            m365_client_id: std::env::var("IRIS_M365_CLIENT_ID").ok(),
            oauth_redirect_port: std::env::var("IRIS_OAUTH_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(8749),
        }
    }
}

/// Initialize the application: create data directory, database pool, and run migrations.
pub fn init(app: &mut tauri::App) -> anyhow::Result<()> {
    let app_data_dir = app.path().app_data_dir()?;
    std::fs::create_dir_all(&app_data_dir)?;
    let db_path = app_data_dir.join("iris-mail.db");
    let database_url = format!("sqlite:{}?mode=rwc", db_path.display());

    tracing::info!("Database path: {}", db_path.display());

    let pool = tauri::async_runtime::block_on(iris_db::create_pool(&database_url))?;

    tracing::info!("Database initialized, migrations applied");

    app.manage(pool);
    app.manage(AppConfig::from_env());

    Ok(())
}
