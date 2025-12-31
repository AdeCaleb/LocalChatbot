//! Tauri commands module - exposes Rust functions to the frontend.
//!
//! Commands are the bridge between your TypeScript/React frontend and Rust backend.
//! The `#[tauri::command]` macro generates the IPC glue code automatically.

use crate::db::{ChatWithMessages, Database, Message};
use chrono::Utc;
use std::sync::Mutex;
use tauri::State;
use uuid::Uuid;

/// Wrapper for thread-safe database access.
///
/// Why `Mutex<Database>`?
/// - Tauri commands can be called from multiple threads
/// - SQLite connections aren't thread-safe by default
/// - Mutex ensures only one thread accesses the database at a time
///
/// Why wrap in a struct?
/// - Makes the State type more readable
/// - Allows adding more fields later if needed (e.g., connection pool)
pub struct DbState(pub Mutex<Database>);

/// Creates a new chat conversation.
///
/// `State<'_, DbState>` is Tauri's dependency injection - it provides
/// access to the database we'll set up in main.rs.
///
/// The `'_` is a lifetime elision - Rust figures out the correct lifetime.
#[tauri::command]
pub fn create_chat(db: State<'_, DbState>) -> Result<ChatWithMessages, String> {
    // Lock the mutex to get exclusive database access
    // `.lock()` returns a Result because another thread might have panicked while holding the lock
    // `.map_err()` converts any error to a String for Tauri's error handling
    let db = db.0.lock().map_err(|e| e.to_string())?;

    // Generate a unique ID using UUID v4 (random)
    let id = Uuid::new_v4().to_string();
    let title = "New Conversation".to_string();

    db.create_chat(&id, &title).map_err(|e| e.to_string())?;

    // Return a ChatWithMessages with empty messages array
    Ok(ChatWithMessages {
        id,
        title,
        messages: vec![],
        created_at: Utc::now(),
        updated_at: Utc::now(),
    })
}

/// Gets all chats (without messages, for the sidebar).
#[tauri::command]
pub fn get_all_chats(db: State<'_, DbState>) -> Result<Vec<crate::db::Chat>, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    db.get_all_chats().map_err(|e| e.to_string())
}

/// Gets a single chat with all its messages.
#[tauri::command]
pub fn get_chat(db: State<'_, DbState>, chat_id: String) -> Result<Option<ChatWithMessages>, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    db.get_chat(&chat_id).map_err(|e| e.to_string())
}

/// Deletes a chat and all its messages.
#[tauri::command]
pub fn delete_chat(db: State<'_, DbState>, chat_id: String) -> Result<bool, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    db.delete_chat(&chat_id).map_err(|e| e.to_string())
}

/// Input structure for adding a message.
///
/// Using a dedicated struct for complex inputs is cleaner than many parameters.
/// `#[serde(rename_all = "camelCase")]` converts Rust's snake_case to JS's camelCase.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AddMessageInput {
    pub chat_id: String,
    pub role: String,
    pub content: String,
    pub sources: Option<String>, // JSON string of sources
}

/// Adds a message to a chat.
///
/// Returns the created message so the frontend can update its state.
#[tauri::command]
pub fn add_message(
    db: State<'_, DbState>,
    input: AddMessageInput,
) -> Result<Message, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;

    let message = Message {
        id: Uuid::new_v4().to_string(),
        chat_id: input.chat_id,
        role: input.role,
        content: input.content,
        timestamp: Utc::now(),
        sources: input.sources,
    };

    db.add_message(&message).map_err(|e| e.to_string())?;

    Ok(message)
}

/// Updates a chat's title.
#[tauri::command]
pub fn update_chat_title(
    db: State<'_, DbState>,
    chat_id: String,
    title: String,
) -> Result<(), String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    db.update_chat_title(&chat_id, &title).map_err(|e| e.to_string())
}

/// Basic chat command - placeholder for future RAG integration.
///
/// Currently just echoes the message. Will be replaced with:
/// 1. Embed the question
/// 2. Search vector store
/// 3. Build context prompt
/// 4. Generate response with LLM
#[tauri::command]
pub async fn chat(message: String) -> Result<String, String> {
    // Placeholder response - will integrate RAG + LLM later
    Ok(format!("Echo: {}", message))
}
