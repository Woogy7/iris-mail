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
            commands::folders::list_folders,
            commands::messages::list_messages,
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Iris Mail");
}
