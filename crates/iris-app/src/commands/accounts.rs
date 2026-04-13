/// List all configured email accounts.
#[tauri::command]
pub async fn list_accounts(
    pool: tauri::State<'_, sqlx::SqlitePool>,
) -> Result<Vec<iris_core::Account>, String> {
    iris_db::repo::AccountRepo::list(&pool)
        .await
        .map_err(|e| e.to_string())
}
