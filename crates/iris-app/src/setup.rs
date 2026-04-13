use tauri::Manager;

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
    Ok(())
}
