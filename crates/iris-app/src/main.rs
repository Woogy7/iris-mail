#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
#[allow(dead_code)]
mod events;
mod setup;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "iris_app=debug,iris_db=debug".into()),
        )
        .init();

    tauri::Builder::default()
        .setup(|app| {
            setup::init(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::accounts::list_accounts,
            commands::accounts::get_account,
            commands::accounts::add_m365_account,
            commands::accounts::add_imap_account,
            commands::accounts::remove_account,
            commands::folders::list_folders,
            commands::folders::sync_folders,
            commands::messages::list_messages,
            commands::messages::fetch_folder_messages,
            commands::messages::get_message_body,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Iris Mail");
}
