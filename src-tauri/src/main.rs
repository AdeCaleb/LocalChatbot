// Prevents additional console window on Windows in release mode
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod chunker;
mod commands;
mod db;
mod documents;
mod embeddings;
mod vector_store;

use commands::{
    add_message, chat, create_chat, delete_chat, get_all_chats, get_chat, update_chat_title,
    // Document commands
    delete_document_cmd, get_all_documents, get_document_content, upload_document,
    // Chunk commands
    get_chunk_stats, get_document_chunks,
    // Embedding commands
    get_embedding_stats, index_all_documents, index_document, init_embedding_model,
    is_model_loaded, search_documents,
    AppPaths, DbState, EmbeddingState,
};
use db::Database;
use std::sync::Mutex;
// Manager trait provides `path()` and `manage()` methods on App
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        // Dialog plugin for file picker dialogs
        .plugin(tauri_plugin_dialog::init())
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

            // Create documents subdirectory for storing uploaded files
            let documents_dir = app_data_dir.join("documents");
            std::fs::create_dir_all(&documents_dir)
                .expect("Failed to create documents directory");

            println!("App data directory: {:?}", app_data_dir);
            println!("Documents directory: {:?}", documents_dir);

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

            // Register app paths
            app.manage(AppPaths { documents_dir });

            // Register embedding model state (initially empty, loaded on demand)
            app.manage(EmbeddingState(Mutex::new(None)));

            Ok(())
        })
        // Register all commands that the frontend can invoke
        .invoke_handler(tauri::generate_handler![
            // Chat commands
            chat,
            create_chat,
            get_all_chats,
            get_chat,
            delete_chat,
            add_message,
            update_chat_title,
            // Document commands
            get_all_documents,
            upload_document,
            delete_document_cmd,
            get_document_content,
            // Chunk commands
            get_document_chunks,
            get_chunk_stats,
            // Embedding commands
            init_embedding_model,
            is_model_loaded,
            index_document,
            index_all_documents,
            search_documents,
            get_embedding_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
