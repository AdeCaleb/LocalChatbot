// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod db;

use commands::{
    add_message, chat, create_chat, delete_chat, get_all_chats, get_chat, update_chat_title,
    DbState,
};
use db::Database;
use std::sync::Mutex;
// Manager trait provides `path()` and `manage()` methods on App
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Setup hook runs once when the app starts
        // This is where we initialize resources like the database
        .setup(|app| {
            // Get the app's data directory - this is where user data should be stored
            // On Linux: ~/.local/share/<app-identifier>/
            // On macOS: ~/Library/Application Support/<app-identifier>/
            // On Windows: C:\Users\<User>\AppData\Roaming\<app-identifier>\
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("Failed to get app data directory");

            // Create the directory if it doesn't exist
            std::fs::create_dir_all(&app_data_dir)
                .expect("Failed to create app data directory");

            // Database file path
            let db_path = app_data_dir.join("chat_history.db");

            println!("Database location: {:?}", db_path);

            // Initialize the database
            // The `expect` will panic with our message if database creation fails
            // In production, you might want more graceful error handling
            let database = Database::new(&db_path)
                .expect("Failed to initialize database");

            // Register the database as managed state
            // Tauri will make this available to any command that requests State<DbState>
            app.manage(DbState(Mutex::new(database)));

            Ok(())
        })
        // Register all commands that the frontend can invoke
        .invoke_handler(tauri::generate_handler![
            chat,
            create_chat,
            get_all_chats,
            get_chat,
            delete_chat,
            add_message,
            update_chat_title,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
