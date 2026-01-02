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

// ============================================================================
// Document Commands
// ============================================================================

use crate::chunker::{self, Chunk, ChunkConfig};
use crate::documents::{self, Document};
use std::path::PathBuf;

/// Application state for storing the documents directory path.
pub struct AppPaths {
    pub documents_dir: PathBuf,
}

/// Response type for document operations (matches frontend expectations).
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentResponse {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub doc_type: String,
    pub size: u64,
    pub uploaded_at: String,
}

impl From<Document> for DocumentResponse {
    fn from(doc: Document) -> Self {
        DocumentResponse {
            id: doc.id,
            name: doc.name,
            doc_type: doc.doc_type.as_str().to_string(),
            size: doc.size,
            uploaded_at: doc.uploaded_at.to_rfc3339(),
        }
    }
}

/// Get all documents.
#[tauri::command]
pub fn get_all_documents(db: State<'_, DbState>) -> Result<Vec<DocumentResponse>, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    let docs = documents::get_all_documents(&db.conn).map_err(|e| e.to_string())?;
    Ok(docs.into_iter().map(DocumentResponse::from).collect())
}

/// Upload and process a document from a file path.
///
/// This command:
/// 1. Reads the file from the given path
/// 2. Extracts text content based on file type
/// 3. Copies the file to the app's documents directory
/// 4. Saves metadata and content to the database
#[tauri::command]
pub fn upload_document(
    db: State<'_, DbState>,
    paths: State<'_, AppPaths>,
    file_path: String,
) -> Result<DocumentResponse, String> {
    let source_path = PathBuf::from(&file_path);

    // Validate the file exists
    if !source_path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Generate a unique ID
    let id = Uuid::new_v4().to_string();

    // Load and extract text from the document
    let loaded = documents::load_document(&source_path, &id)
        .map_err(|e| e.to_string())?;

    // Copy the file to our documents directory for safekeeping
    let file_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("document");

    let dest_path = paths.documents_dir.join(format!("{}_{}", id, file_name));
    std::fs::copy(&source_path, &dest_path)
        .map_err(|e| format!("Failed to copy file: {}", e))?;

    // Update the document metadata with the new path
    let mut doc = loaded.metadata;
    doc.path = dest_path.to_string_lossy().to_string();

    // Save to database
    let db = db.0.lock().map_err(|e| e.to_string())?;
    documents::save_document(&db.conn, &doc).map_err(|e| e.to_string())?;
    documents::save_document_content(&db.conn, &doc.id, &loaded.content)
        .map_err(|e| e.to_string())?;

    // Chunk the document for RAG
    let config = ChunkConfig::default();
    let chunks = chunker::chunk_text(&doc.id, &loaded.content, &config);
    chunker::save_chunks(&db.conn, &chunks).map_err(|e| e.to_string())?;

    println!(
        "Uploaded document: {} ({} bytes, {} chars of text, {} chunks)",
        doc.name,
        doc.size,
        loaded.content.len(),
        chunks.len()
    );

    Ok(DocumentResponse::from(doc))
}

/// Delete a document.
#[tauri::command]
pub fn delete_document_cmd(
    db: State<'_, DbState>,
    document_id: String,
) -> Result<bool, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;

    // Get the document to find its file path
    if let Some(doc) = documents::get_document(&db.conn, &document_id)
        .map_err(|e| e.to_string())?
    {
        // Delete the file from disk
        let path = PathBuf::from(&doc.path);
        if path.exists() {
            std::fs::remove_file(&path).ok(); // Ignore errors if file can't be deleted
        }
    }

    // Delete from database
    documents::delete_document(&db.conn, &document_id).map_err(|e| e.to_string())
}

/// Get document content (extracted text).
#[tauri::command]
pub fn get_document_content(
    db: State<'_, DbState>,
    document_id: String,
) -> Result<Option<String>, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    documents::get_document_content(&db.conn, &document_id).map_err(|e| e.to_string())
}

// ============================================================================
// Chunk Commands
// ============================================================================

/// Response type for chunks.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChunkResponse {
    pub id: String,
    pub document_id: String,
    pub chunk_index: usize,
    pub content: String,
    pub start_offset: usize,
    pub end_offset: usize,
}

impl From<Chunk> for ChunkResponse {
    fn from(chunk: Chunk) -> Self {
        ChunkResponse {
            id: chunk.id,
            document_id: chunk.document_id,
            chunk_index: chunk.chunk_index,
            content: chunk.content,
            start_offset: chunk.start_offset,
            end_offset: chunk.end_offset,
        }
    }
}

/// Get all chunks for a document.
#[tauri::command]
pub fn get_document_chunks(
    db: State<'_, DbState>,
    document_id: String,
) -> Result<Vec<ChunkResponse>, String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    let chunks = chunker::get_document_chunks(&db.conn, &document_id)
        .map_err(|e| e.to_string())?;
    Ok(chunks.into_iter().map(ChunkResponse::from).collect())
}

/// Get chunk statistics.
#[tauri::command]
pub fn get_chunk_stats(db: State<'_, DbState>) -> Result<(usize, usize), String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    chunker::get_chunk_stats(&db.conn).map_err(|e| e.to_string())
}
