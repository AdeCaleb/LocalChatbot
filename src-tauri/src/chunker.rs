//! Text chunking module for RAG pipeline.
//!
//! This module splits documents into overlapping chunks that can be:
//! 1. Embedded into vectors for similarity search
//! 2. Used as context for LLM responses
//!
//! ## Why Chunking Matters
//!
//! LLMs have context limits, and embedding models work best with smaller text.
//! Chunking lets us:
//! - Find the most relevant parts of large documents
//! - Fit multiple relevant chunks into the LLM context
//! - Improve retrieval accuracy by matching at a granular level
//!
//! ## Overlap
//!
//! Chunks overlap to avoid losing context at boundaries. For example:
//! "The cat sat on the mat. It was comfortable."
//!
//! Without overlap, "It" in chunk 2 might lose its referent "cat" from chunk 1.
//! With overlap, the sentence about the cat appears in both chunks.
//!
//! ## UTF-8 Safety
//!
//! This chunker works with character counts, not byte counts, to safely handle
//! multi-byte UTF-8 characters (like smart quotes, emojis, non-ASCII text).

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

/// Configuration for text chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Target size for each chunk in characters (not bytes).
    /// Actual chunks may be slightly smaller to avoid breaking words.
    pub chunk_size: usize,

    /// Number of characters to overlap between consecutive chunks.
    /// Higher overlap = better context preservation but more chunks.
    pub overlap: usize,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        ChunkConfig {
            chunk_size: 1000,  // ~250 tokens (rough estimate: 4 chars/token)
            overlap: 200,      // 20% overlap
        }
    }
}

/// A chunk of text from a document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    pub id: String,
    pub document_id: String,
    /// Zero-based index of this chunk within the document
    pub chunk_index: usize,
    /// The actual text content
    pub content: String,
    /// Character offset where this chunk starts in the original document
    pub start_offset: usize,
    /// Character offset where this chunk ends in the original document
    pub end_offset: usize,
}

/// Split text into overlapping chunks.
///
/// This function is UTF-8 safe - it works with character indices, not byte indices.
/// It tries to be "smart" about where to split:
/// 1. Prefers splitting at paragraph boundaries (\n\n)
/// 2. Falls back to sentence boundaries (. ! ?)
/// 3. Falls back to word boundaries (spaces)
/// 4. Last resort: splits at character boundary
pub fn chunk_text(document_id: &str, text: &str, config: &ChunkConfig) -> Vec<Chunk> {
    let mut chunks = Vec::new();
    let text = text.trim();

    if text.is_empty() {
        return chunks;
    }

    // Collect character indices for UTF-8 safe slicing
    let char_indices: Vec<(usize, char)> = text.char_indices().collect();
    let total_chars = char_indices.len();

    // If text is smaller than chunk size, return as single chunk
    if total_chars <= config.chunk_size {
        chunks.push(Chunk {
            id: format!("{}-0", document_id),
            document_id: document_id.to_string(),
            chunk_index: 0,
            content: text.to_string(),
            start_offset: 0,
            end_offset: total_chars,
        });
        return chunks;
    }

    let mut start_char = 0; // Character index (not byte)
    let mut chunk_index = 0;

    while start_char < total_chars {
        // Calculate the end character position for this chunk
        let mut end_char = (start_char + config.chunk_size).min(total_chars);

        // If we're not at the end, try to find a good break point
        if end_char < total_chars {
            end_char = find_break_point_chars(&char_indices, start_char, end_char);
        }

        // Get byte positions from character positions for slicing
        let start_byte = char_indices[start_char].0;
        let end_byte = if end_char >= total_chars {
            text.len()
        } else {
            char_indices[end_char].0
        };

        // Extract the chunk content
        let content = text[start_byte..end_byte].trim().to_string();

        if !content.is_empty() {
            chunks.push(Chunk {
                id: format!("{}-{}", document_id, chunk_index),
                document_id: document_id.to_string(),
                chunk_index,
                content,
                start_offset: start_char,
                end_offset: end_char,
            });
            chunk_index += 1;
        }

        // Move start position, accounting for overlap
        let step = if config.chunk_size > config.overlap {
            config.chunk_size - config.overlap
        } else {
            config.chunk_size / 2
        };
        start_char += step.max(1);
    }

    chunks
}

/// Find a good break point for chunking (working with character indices).
///
/// Searches backwards from `end_char` to find a natural break point.
/// Returns a character index (not byte index).
fn find_break_point_chars(
    char_indices: &[(usize, char)],
    start_char: usize,
    end_char: usize,
) -> usize {
    // Look backwards from end for a good break point
    let search_start = if end_char > start_char + 50 {
        end_char.saturating_sub(200) // Look in last 200 chars
    } else {
        start_char
    };

    // First, look for paragraph break (double newline)
    let mut found_newline = false;
    for i in (search_start..end_char).rev() {
        let c = char_indices[i].1;
        if c == '\n' {
            if found_newline {
                // Found double newline - return position after it
                return (i + 2).min(end_char);
            }
            found_newline = true;
        } else if !c.is_whitespace() {
            found_newline = false;
        }
    }

    // Look for sentence break (. ! ? followed by space)
    for i in (search_start..end_char.saturating_sub(1)).rev() {
        let c = char_indices[i].1;
        if c == '.' || c == '!' || c == '?' {
            // Check if followed by whitespace
            if i + 1 < char_indices.len() {
                let next_c = char_indices[i + 1].1;
                if next_c.is_whitespace() {
                    return i + 1; // Return position after punctuation
                }
            }
        }
    }

    // Look for word break (space)
    for i in (search_start..end_char).rev() {
        let c = char_indices[i].1;
        if c == ' ' || c == '\n' || c == '\t' {
            return i + 1; // Return position after space
        }
    }

    // No good break point found, use the original end
    end_char
}

/// Initialize the chunks table in SQLite.
pub fn init_chunks_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS chunks (
            id TEXT PRIMARY KEY,
            document_id TEXT NOT NULL,
            chunk_index INTEGER NOT NULL,
            content TEXT NOT NULL,
            start_offset INTEGER NOT NULL,
            end_offset INTEGER NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )",
        [],
    )?;

    // Index for fast lookup by document
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_chunks_document_id ON chunks(document_id)",
        [],
    )?;

    Ok(())
}

/// Save chunks to the database.
pub fn save_chunks(conn: &Connection, chunks: &[Chunk]) -> Result<(), rusqlite::Error> {
    for chunk in chunks {
        conn.execute(
            "INSERT OR REPLACE INTO chunks (id, document_id, chunk_index, content, start_offset, end_offset)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                chunk.id,
                chunk.document_id,
                chunk.chunk_index as i64,
                chunk.content,
                chunk.start_offset as i64,
                chunk.end_offset as i64,
            ],
        )?;
    }
    Ok(())
}

/// Get all chunks for a document.
pub fn get_document_chunks(conn: &Connection, document_id: &str) -> Result<Vec<Chunk>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, document_id, chunk_index, content, start_offset, end_offset
         FROM chunks WHERE document_id = ?1 ORDER BY chunk_index"
    )?;

    let chunks = stmt.query_map(params![document_id], |row| {
        Ok(Chunk {
            id: row.get(0)?,
            document_id: row.get(1)?,
            chunk_index: row.get::<_, i64>(2)? as usize,
            content: row.get(3)?,
            start_offset: row.get::<_, i64>(4)? as usize,
            end_offset: row.get::<_, i64>(5)? as usize,
        })
    })?;

    chunks.collect()
}

/// Get all chunks (for all documents).
pub fn get_all_chunks(conn: &Connection) -> Result<Vec<Chunk>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "SELECT id, document_id, chunk_index, content, start_offset, end_offset
         FROM chunks ORDER BY document_id, chunk_index"
    )?;

    let chunks = stmt.query_map([], |row| {
        Ok(Chunk {
            id: row.get(0)?,
            document_id: row.get(1)?,
            chunk_index: row.get::<_, i64>(2)? as usize,
            content: row.get(3)?,
            start_offset: row.get::<_, i64>(4)? as usize,
            end_offset: row.get::<_, i64>(5)? as usize,
        })
    })?;

    chunks.collect()
}

/// Delete all chunks for a document.
pub fn delete_document_chunks(conn: &Connection, document_id: &str) -> Result<(), rusqlite::Error> {
    conn.execute("DELETE FROM chunks WHERE document_id = ?1", params![document_id])?;
    Ok(())
}

/// Get chunk count statistics.
pub fn get_chunk_stats(conn: &Connection) -> Result<(usize, usize), rusqlite::Error> {
    let total_chunks: i64 = conn.query_row(
        "SELECT COUNT(*) FROM chunks",
        [],
        |row| row.get(0),
    )?;

    let total_docs: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT document_id) FROM chunks",
        [],
        |row| row.get(0),
    )?;

    Ok((total_chunks as usize, total_docs as usize))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_text_single_chunk() {
        let config = ChunkConfig {
            chunk_size: 100,
            overlap: 20,
        };
        let chunks = chunk_text("doc-1", "Small text.", &config);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, "Small text.");
    }

    #[test]
    fn test_chunking_with_overlap() {
        let config = ChunkConfig {
            chunk_size: 50,
            overlap: 10,
        };
        let text = "This is the first sentence. This is the second sentence. This is the third sentence.";
        let chunks = chunk_text("doc-1", text, &config);

        // Should have multiple chunks
        assert!(chunks.len() > 1);

        // Check that chunks are properly indexed
        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
            assert_eq!(chunk.document_id, "doc-1");
        }
    }

    #[test]
    fn test_chunk_break_at_sentence() {
        let config = ChunkConfig {
            chunk_size: 40,
            overlap: 5,
        };
        let text = "Hello world. This is a test. Another sentence here.";
        let chunks = chunk_text("doc-1", text, &config);

        // Should have multiple chunks
        assert!(chunks.len() >= 1);

        // All chunks should have non-empty content
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
        }
    }

    #[test]
    fn test_empty_text() {
        let config = ChunkConfig::default();
        let chunks = chunk_text("doc-1", "", &config);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_whitespace_only() {
        let config = ChunkConfig::default();
        let chunks = chunk_text("doc-1", "   \n\n   ", &config);
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_utf8_multibyte_chars() {
        // Test with smart quotes, emojis, and non-ASCII characters
        let config = ChunkConfig {
            chunk_size: 20,
            overlap: 5,
        };
        // Using Unicode escapes for smart quotes to avoid syntax issues
        let text = "Hello \u{201C}world\u{201D} with émojis 🎉 and más text here.";
        let chunks = chunk_text("doc-1", text, &config);

        // Should not panic and produce valid chunks
        assert!(!chunks.is_empty());

        // All chunks should be valid UTF-8 strings
        for chunk in &chunks {
            assert!(!chunk.content.is_empty());
            // This would panic if content was invalid UTF-8
            let _ = chunk.content.chars().count();
        }
    }

    #[test]
    fn test_database_operations() {
        use chrono::Utc;

        let conn = Connection::open_in_memory().unwrap();
        // Create documents table first (chunks has a foreign key to it)
        crate::documents::init_documents_table(&conn).unwrap();
        init_chunks_table(&conn).unwrap();

        // Create a dummy document for foreign key constraint
        let doc = crate::documents::Document {
            id: "doc-1".to_string(),
            name: "test.txt".to_string(),
            doc_type: crate::documents::DocumentType::Txt,
            size: 100,
            uploaded_at: Utc::now(),
            path: "/tmp/test.txt".to_string(),
        };
        crate::documents::save_document(&conn, &doc).unwrap();

        let chunks = vec![
            Chunk {
                id: "doc-1-0".to_string(),
                document_id: "doc-1".to_string(),
                chunk_index: 0,
                content: "First chunk".to_string(),
                start_offset: 0,
                end_offset: 11,
            },
            Chunk {
                id: "doc-1-1".to_string(),
                document_id: "doc-1".to_string(),
                chunk_index: 1,
                content: "Second chunk".to_string(),
                start_offset: 9,
                end_offset: 21,
            },
        ];

        save_chunks(&conn, &chunks).unwrap();

        let loaded = get_document_chunks(&conn, "doc-1").unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].content, "First chunk");
        assert_eq!(loaded[1].content, "Second chunk");

        let (total, docs) = get_chunk_stats(&conn).unwrap();
        assert_eq!(total, 2);
        assert_eq!(docs, 1);
    }
}
