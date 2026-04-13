/// List folders for a given account.
#[tauri::command]
pub async fn list_folders(
    pool: tauri::State<'_, sqlx::SqlitePool>,
    account_id: String,
) -> Result<Vec<iris_core::Folder>, String> {
    let id = iris_core::AccountId(uuid::Uuid::parse_str(&account_id).map_err(|e| e.to_string())?);
    iris_db::repo::FolderRepo::list_by_account(&pool, &id)
        .await
        .map_err(|e| e.to_string())
}
