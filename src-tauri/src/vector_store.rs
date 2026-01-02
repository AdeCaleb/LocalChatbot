//! Vector store for semantic similarity search.
//!
//! This module stores embeddings and provides fast similarity search
//! for the RAG pipeline. It uses SQLite for persistence and an in-memory
//! index for fast searches.
//!
//! ## Architecture
//!
//! - Embeddings are stored in SQLite as BLOBs (binary data)
//! - On search, embeddings are loaded into memory for fast comparison
//! - Cosine similarity is used for ranking results
//!
//! ## Why Simple Brute-Force?
//!
//! For collections under ~10,000 chunks, linear search is fast enough
//! (milliseconds) and has zero complexity. More sophisticated indexes
//! (HNSW, IVF) add complexity and are only needed at larger scale.

use crate::embeddings::{cosine_similarity, EMBEDDING_DIM};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// A search result with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The chunk ID
    pub chunk_id: String,
    /// The document ID this chunk belongs to
    pub document_id: String,
    /// The actual text content
    pub content: String,
    /// Cosine similarity score (0.0 to 1.0, higher = more similar)
    pub score: f32,
}

/// Initialize the embeddings table in SQLite.
///
/// Stores chunk embeddings as binary BLOBs for efficient storage.
pub fn init_embeddings_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS embeddings (
            chunk_id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            embedding BLOB NOT NULL,
            FOREIGN KEY (chunk_id) REFERENCES chunks(id) ON DELETE CASCADE,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Index for fast lookup by document
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_embeddings_document_id ON embeddings(document_id)",
        [],
    )?;

    Ok(())
}

/// Save an embedding for a chunk.
///
/// The embedding is stored as a BLOB (binary large object).
/// SQLite handles the binary data efficiently.
pub fn save_embedding(
    conn: &Connection,
    chunk_id: &str,
    document_id: &str,
    embedding: &[f32],
) -> Result<(), rusqlite::Error> {
    // Convert f32 slice to bytes
    let bytes = embedding_to_bytes(embedding);

    conn.execute(
        "INSERT OR REPLACE INTO embeddings (chunk_id, document_id, embedding)
         VALUES (?1, ?2, ?3)",
        params![chunk_id, document_id, bytes],
    )?;

    Ok(())
}

/// Get the embedding for a specific chunk.
pub fn get_embedding(conn: &Connection, chunk_id: &str) -> Result<Option<Vec<f32>>, rusqlite::Error> {
    let mut stmt = conn.prepare("SELECT embedding FROM embeddings WHERE chunk_id = ?1")?;

    let result = stmt.query_row(params![chunk_id], |row| {
        let bytes: Vec<u8> = row.get(0)?;
        Ok(bytes_to_embedding(&bytes))
    });

    match result {
        Ok(embedding) => Ok(Some(embedding)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Search for similar chunks using cosine similarity.
///
/// Returns the top `k` most similar chunks to the query embedding.
///
/// ## Algorithm
///
/// 1. Load all embeddings from the database
/// 2. Compute cosine similarity with the query
/// 3. Sort by similarity (descending)
/// 4. Return top k results
pub fn search_similar(
    conn: &Connection,
    query_embedding: &[f32],
    k: usize,
) -> Result<Vec<SearchResult>, rusqlite::Error> {
    // Load all embeddings with their chunk info
    let mut stmt = conn.prepare(
        "SELECT e.chunk_id, e.document_id, e.embedding, c.content
         FROM embeddings e
         JOIN chunks c ON e.chunk_id = c.id"
    )?;

    let mut results: Vec<SearchResult> = stmt
        .query_map([], |row| {
            let chunk_id: String = row.get(0)?;
            let document_id: String = row.get(1)?;
            let bytes: Vec<u8> = row.get(2)?;
            let content: String = row.get(3)?;

            let embedding = bytes_to_embedding(&bytes);
            let score = cosine_similarity(query_embedding, &embedding);

            Ok(SearchResult {
                chunk_id,
                document_id,
                content,
                score,
            })
        })?
        .filter_map(|r| r.ok())
        .collect();

    // Sort by score descending
    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

    // Return top k
    results.truncate(k);

    Ok(results)
}

/// Delete embeddings for a document.
///
/// Called when a document is deleted to clean up its embeddings.
pub fn delete_document_embeddings(conn: &Connection, document_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM embeddings WHERE document_id = ?1",
        params![document_id],
    )?;
    Ok(())
}

/// Get statistics about stored embeddings.
pub fn get_embedding_stats(conn: &Connection) -> Result<(usize, usize), rusqlite::Error> {
    let total_embeddings: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embeddings",
        [],
        |row| row.get(0),
    )?;

    let total_docs: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT document_id) FROM embeddings",
        [],
        |row| row.get(0),
    )?;

    Ok((total_embeddings as usize, total_docs as usize))
}

/// Check if embeddings exist for a chunk.
pub fn has_embedding(conn: &Connection, chunk_id: &str) -> Result<bool, rusqlite::Error> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM embeddings WHERE chunk_id = ?1",
        params![chunk_id],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

/// Convert a f32 embedding to bytes for SQLite storage.
///
/// Uses little-endian byte order for consistency.
fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
    embedding
        .iter()
        .flat_map(|f| f.to_le_bytes())
        .collect()
}

/// Convert bytes from SQLite back to f32 embedding.
fn bytes_to_embedding(bytes: &[u8]) -> Vec<f32> {
    bytes
        .chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedding_bytes_roundtrip() {
        let original: Vec<f32> = vec![0.1, 0.2, -0.3, 0.4, 0.5];
        let bytes = embedding_to_bytes(&original);
        let recovered = bytes_to_embedding(&bytes);

        assert_eq!(original.len(), recovered.len());
        for (a, b) in original.iter().zip(recovered.iter()) {
            assert!((a - b).abs() < 1e-7, "Mismatch: {} != {}", a, b);
        }
    }

    #[test]
    fn test_database_operations() {
        use chrono::Utc;

        let conn = Connection::open_in_memory().unwrap();

        // Set up all required tables
        crate::documents::init_documents_table(&conn).unwrap();
        crate::chunker::init_chunks_table(&conn).unwrap();
        init_embeddings_table(&conn).unwrap();

        // Create a document
        let doc = crate::documents::Document {
            id: "doc-1".to_string(),
            name: "test.txt".to_string(),
            doc_type: crate::documents::DocumentType::Txt,
            size: 100,
            uploaded_at: Utc::now(),
            path: "/tmp/test.txt".to_string(),
        };
        crate::documents::save_document(&conn, &doc).unwrap();

        // Create a chunk
        let chunk = crate::chunker::Chunk {
            id: "doc-1-0".to_string(),
            document_id: "doc-1".to_string(),
            chunk_index: 0,
            content: "Test content".to_string(),
            start_offset: 0,
            end_offset: 12,
        };
        crate::chunker::save_chunks(&conn, &[chunk]).unwrap();

        // Save an embedding
        let embedding: Vec<f32> = (0..EMBEDDING_DIM).map(|i| i as f32 / EMBEDDING_DIM as f32).collect();
        save_embedding(&conn, "doc-1-0", "doc-1", &embedding).unwrap();

        // Retrieve the embedding
        let retrieved = get_embedding(&conn, "doc-1-0").unwrap().unwrap();
        assert_eq!(retrieved.len(), EMBEDDING_DIM);

        // Check stats
        let (total, docs) = get_embedding_stats(&conn).unwrap();
        assert_eq!(total, 1);
        assert_eq!(docs, 1);

        // Search (should find the chunk)
        let results = search_similar(&conn, &embedding, 10).unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].score > 0.99); // Should be very similar to itself
    }
}
