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
/// 5. Chunks the text and generates embeddings (if model is loaded)
#[tauri::command]
pub async fn upload_document(
    db: State<'_, DbState>,
    paths: State<'_, AppPaths>,
    model: State<'_, EmbeddingState>,
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

    // Generate embeddings if model is loaded
    let mut embeddings_count = 0;
    {
        let model_guard = model.0.lock().map_err(|e| e.to_string())?;
        if let Some(embedding_model) = model_guard.as_ref() {
            // Generate embeddings for all chunks
            let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
            match embedding_model.encode_batch(&texts) {
                Ok(embeddings) => {
                    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
                        vector_store::save_embedding(&db.conn, &chunk.id, &doc.id, embedding)
                            .map_err(|e| e.to_string())?;
                    }
                    embeddings_count = chunks.len();
                }
                Err(e) => {
                    println!("Warning: Failed to generate embeddings: {}", e);
                }
            }
        }
    }

    println!(
        "Uploaded document: {} ({} bytes, {} chars, {} chunks, {} embeddings)",
        doc.name,
        doc.size,
        loaded.content.len(),
        chunks.len(),
        embeddings_count
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

// ============================================================================
// Embedding Commands
// ============================================================================

use crate::embeddings::EmbeddingModel;
use crate::vector_store::{self, SearchResult};

/// Wrapper for thread-safe embedding model access.
///
/// The model is wrapped in Option because it's loaded on-demand,
/// not at startup (to avoid slow app launch).
pub struct EmbeddingState(pub Mutex<Option<EmbeddingModel>>);

/// Initialize the embedding model.
///
/// Downloads the model from Hugging Face if not cached (~90MB).
/// This should be called before indexing or searching.
#[tauri::command]
pub async fn init_embedding_model(model: State<'_, EmbeddingState>) -> Result<String, String> {
    // Check if already loaded
    {
        let guard = model.0.lock().map_err(|e| e.to_string())?;
        if guard.is_some() {
            return Ok("Model already loaded".to_string());
        }
    }

    // Load the model (this might download it)
    // Run in blocking task since model loading is CPU-intensive
    let loaded_model = tokio::task::spawn_blocking(|| {
        EmbeddingModel::new()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| e.to_string())?;

    // Store in state
    let mut guard = model.0.lock().map_err(|e| e.to_string())?;
    *guard = Some(loaded_model);

    Ok("Model loaded successfully".to_string())
}

/// Check if the embedding model is loaded.
#[tauri::command]
pub fn is_model_loaded(model: State<'_, EmbeddingState>) -> Result<bool, String> {
    let guard = model.0.lock().map_err(|e| e.to_string())?;
    Ok(guard.is_some())
}

/// Index a document by generating embeddings for all its chunks.
///
/// Must call `init_embedding_model` first.
#[tauri::command]
pub async fn index_document(
    db: State<'_, DbState>,
    model: State<'_, EmbeddingState>,
    document_id: String,
) -> Result<usize, String> {
    // Get the embedding model
    let model_guard = model.0.lock().map_err(|e| e.to_string())?;
    let embedding_model = model_guard
        .as_ref()
        .ok_or("Embedding model not loaded. Call init_embedding_model first.")?;

    // Get all chunks for this document
    let db_guard = db.0.lock().map_err(|e| e.to_string())?;
    let chunks = chunker::get_document_chunks(&db_guard.conn, &document_id)
        .map_err(|e| e.to_string())?;

    if chunks.is_empty() {
        return Ok(0);
    }

    // Generate embeddings for all chunks
    let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedding_model
        .encode_batch(&texts)
        .map_err(|e| e.to_string())?;

    // Save embeddings to database
    for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
        vector_store::save_embedding(&db_guard.conn, &chunk.id, &document_id, embedding)
            .map_err(|e| e.to_string())?;
    }

    let count = chunks.len();
    println!(
        "Indexed document {} with {} chunk embeddings",
        document_id, count
    );

    Ok(count)
}

/// Search for chunks similar to a query.
///
/// Returns the top k most similar chunks across all documents.
#[tauri::command]
pub async fn search_documents(
    db: State<'_, DbState>,
    model: State<'_, EmbeddingState>,
    query: String,
    top_k: Option<usize>,
) -> Result<Vec<SearchResult>, String> {
    let k = top_k.unwrap_or(5);

    // Get the embedding model
    let model_guard = model.0.lock().map_err(|e| e.to_string())?;
    let embedding_model = model_guard
        .as_ref()
        .ok_or("Embedding model not loaded. Call init_embedding_model first.")?;

    // Embed the query
    let query_embedding = embedding_model
        .encode(&query)
        .map_err(|e| e.to_string())?;

    // Search for similar chunks
    let db_guard = db.0.lock().map_err(|e| e.to_string())?;
    let results = vector_store::search_similar(&db_guard.conn, &query_embedding, k)
        .map_err(|e| e.to_string())?;

    Ok(results)
}

/// Get embedding statistics.
#[tauri::command]
pub fn get_embedding_stats(db: State<'_, DbState>) -> Result<(usize, usize), String> {
    let db = db.0.lock().map_err(|e| e.to_string())?;
    vector_store::get_embedding_stats(&db.conn).map_err(|e| e.to_string())
}

/// Index all documents that don't have embeddings yet.
///
/// Useful for indexing documents uploaded before the model was loaded,
/// or after upgrading the app.
#[tauri::command]
pub async fn index_all_documents(
    db: State<'_, DbState>,
    model: State<'_, EmbeddingState>,
) -> Result<(usize, usize), String> {
    // Get the embedding model
    let model_guard = model.0.lock().map_err(|e| e.to_string())?;
    let embedding_model = model_guard
        .as_ref()
        .ok_or("Embedding model not loaded. Call init_embedding_model first.")?;

    let db_guard = db.0.lock().map_err(|e| e.to_string())?;

    // Get all documents
    let docs = documents::get_all_documents(&db_guard.conn).map_err(|e| e.to_string())?;

    let mut total_chunks = 0;
    let mut docs_indexed = 0;

    for doc in &docs {
        // Get chunks for this document
        let chunks = chunker::get_document_chunks(&db_guard.conn, &doc.id)
            .map_err(|e| e.to_string())?;

        if chunks.is_empty() {
            continue;
        }

        // Check if first chunk already has embedding (skip if already indexed)
        if vector_store::has_embedding(&db_guard.conn, &chunks[0].id)
            .map_err(|e| e.to_string())?
        {
            continue;
        }

        // Generate embeddings for all chunks
        let texts: Vec<&str> = chunks.iter().map(|c| c.content.as_str()).collect();
        let embeddings = embedding_model
            .encode_batch(&texts)
            .map_err(|e| e.to_string())?;

        // Save embeddings
        for (chunk, embedding) in chunks.iter().zip(embeddings.iter()) {
            vector_store::save_embedding(&db_guard.conn, &chunk.id, &doc.id, embedding)
                .map_err(|e| e.to_string())?;
        }

        total_chunks += chunks.len();
        docs_indexed += 1;
        println!("Indexed document: {} ({} chunks)", doc.name, chunks.len());
    }

    println!(
        "Indexing complete: {} documents, {} chunks",
        docs_indexed, total_chunks
    );

    Ok((docs_indexed, total_chunks))
}
