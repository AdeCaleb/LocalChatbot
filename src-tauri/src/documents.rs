//! Document loading and management module.
//!
//! This module handles:
//! - Loading documents from disk (PDF, TXT, MD)
//! - Extracting text content from different formats
//! - Storing document metadata in SQLite
//!
//! Key Rust concepts demonstrated:
//! - Enum variants for different document types
//! - Pattern matching for handling different cases
//! - Error handling with custom error types
//! - File I/O operations

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Supported document types.
///
/// Rust enums are powerful - each variant can hold different data.
/// Here we use a simple enum just for type identification.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DocumentType {
    Pdf,
    Txt,
    Md,
}

impl DocumentType {
    /// Determine document type from file extension.
    ///
    /// Returns `None` if the extension isn't supported.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "pdf" => Some(DocumentType::Pdf),
            "txt" => Some(DocumentType::Txt),
            "md" | "markdown" => Some(DocumentType::Md),
            _ => None,
        }
    }

    /// Get the extension string for this document type.
    pub fn as_str(&self) -> &'static str {
        match self {
            DocumentType::Pdf => "pdf",
            DocumentType::Txt => "txt",
            DocumentType::Md => "md",
        }
    }
}

/// Metadata about a document stored in the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub doc_type: DocumentType,
    pub size: u64,
    pub uploaded_at: DateTime<Utc>,
    /// Path where the document is stored
    pub path: String,
}

/// Result of loading a document - includes both metadata and extracted text.
#[derive(Debug)]
pub struct LoadedDocument {
    pub metadata: Document,
    pub content: String,
}

/// Custom error type for document operations.
///
/// Using `thiserror` would be cleaner, but we keep it simple here.
/// This demonstrates how to create custom error types in Rust.
#[derive(Debug)]
pub enum DocumentError {
    IoError(std::io::Error),
    PdfError(String),
    UnsupportedFormat(String),
    DatabaseError(rusqlite::Error),
    NotFound(String),
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DocumentError::IoError(e) => write!(f, "IO error: {}", e),
            DocumentError::PdfError(e) => write!(f, "PDF error: {}", e),
            DocumentError::UnsupportedFormat(ext) => write!(f, "Unsupported format: {}", ext),
            DocumentError::DatabaseError(e) => write!(f, "Database error: {}", e),
            DocumentError::NotFound(id) => write!(f, "Document not found: {}", id),
        }
    }
}

impl std::error::Error for DocumentError {}

// Implement From traits for easy error conversion with `?` operator
impl From<std::io::Error> for DocumentError {
    fn from(e: std::io::Error) -> Self {
        DocumentError::IoError(e)
    }
}

impl From<rusqlite::Error> for DocumentError {
    fn from(e: rusqlite::Error) -> Self {
        DocumentError::DatabaseError(e)
    }
}

/// Initialize the documents table in SQLite.
pub fn init_documents_table(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS documents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            doc_type TEXT NOT NULL,
            size INTEGER NOT NULL,
            uploaded_at TEXT NOT NULL,
            path TEXT NOT NULL
        )",
        [],
    )?;

    // Also create a table to store extracted text content
    // This avoids re-extracting text every time we need it
    conn.execute(
        "CREATE TABLE IF NOT EXISTS document_content (
            document_id TEXT PRIMARY KEY,
            content TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
        )",
        [],
    )?;

    Ok(())
}

/// Extract text from a PDF file.
///
/// PDF extraction can be tricky - not all PDFs have extractable text
/// (e.g., scanned documents). The `pdf-extract` crate handles common cases.
fn extract_pdf_text(path: &Path) -> Result<String, DocumentError> {
    // Read the PDF bytes
    let bytes = fs::read(path)?;

    // Extract text using pdf-extract
    // This crate handles the complexity of PDF parsing
    pdf_extract::extract_text_from_mem(&bytes)
        .map_err(|e| DocumentError::PdfError(e.to_string()))
}

/// Extract text from a plain text or markdown file.
///
/// For TXT and MD files, we simply read the content as UTF-8.
/// Markdown is kept as-is (we don't strip formatting).
fn extract_text_file(path: &Path) -> Result<String, DocumentError> {
    fs::read_to_string(path).map_err(DocumentError::from)
}

/// Load a document from disk and extract its text content.
///
/// This is the main entry point for document loading.
/// It determines the file type, extracts text, and returns both
/// metadata and content.
pub fn load_document(path: &Path, id: &str) -> Result<LoadedDocument, DocumentError> {
    // Get file metadata
    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    // Determine file type from extension
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .ok_or_else(|| DocumentError::UnsupportedFormat("no extension".to_string()))?;

    let doc_type = DocumentType::from_extension(extension)
        .ok_or_else(|| DocumentError::UnsupportedFormat(extension.to_string()))?;

    // Get filename
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Extract text based on document type
    let content = match doc_type {
        DocumentType::Pdf => extract_pdf_text(path)?,
        DocumentType::Txt | DocumentType::Md => extract_text_file(path)?,
    };

    let document = Document {
        id: id.to_string(),
        name,
        doc_type,
        size,
        uploaded_at: Utc::now(),
        path: path.to_string_lossy().to_string(),
    };

    Ok(LoadedDocument {
        metadata: document,
        content,
    })
}

/// Save document metadata to the database.
pub fn save_document(conn: &Connection, doc: &Document) -> Result<(), DocumentError> {
    conn.execute(
        "INSERT INTO documents (id, name, doc_type, size, uploaded_at, path)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            doc.id,
            doc.name,
            doc.doc_type.as_str(),
            doc.size as i64,
            doc.uploaded_at.to_rfc3339(),
            doc.path,
        ],
    )?;
    Ok(())
}

/// Save extracted document content to the database.
pub fn save_document_content(
    conn: &Connection,
    document_id: &str,
    content: &str,
) -> Result<(), DocumentError> {
    conn.execute(
        "INSERT INTO document_content (document_id, content) VALUES (?1, ?2)",
        params![document_id, content],
    )?;
    Ok(())
}

/// Get all documents from the database.
pub fn get_all_documents(conn: &Connection) -> Result<Vec<Document>, DocumentError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, doc_type, size, uploaded_at, path FROM documents ORDER BY uploaded_at DESC"
    )?;

    let docs = stmt.query_map([], |row| {
        let doc_type_str: String = row.get(2)?;
        let doc_type = DocumentType::from_extension(&doc_type_str).unwrap_or(DocumentType::Txt);

        Ok(Document {
            id: row.get(0)?,
            name: row.get(1)?,
            doc_type,
            size: row.get::<_, i64>(3)? as u64,
            uploaded_at: parse_datetime(&row.get::<_, String>(4)?),
            path: row.get(5)?,
        })
    })?;

    docs.collect::<Result<Vec<_>, _>>().map_err(DocumentError::from)
}

/// Get a single document by ID.
pub fn get_document(conn: &Connection, id: &str) -> Result<Option<Document>, DocumentError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, doc_type, size, uploaded_at, path FROM documents WHERE id = ?1"
    )?;

    let result = stmt.query_row(params![id], |row| {
        let doc_type_str: String = row.get(2)?;
        let doc_type = DocumentType::from_extension(&doc_type_str).unwrap_or(DocumentType::Txt);

        Ok(Document {
            id: row.get(0)?,
            name: row.get(1)?,
            doc_type,
            size: row.get::<_, i64>(3)? as u64,
            uploaded_at: parse_datetime(&row.get::<_, String>(4)?),
            path: row.get(5)?,
        })
    });

    match result {
        Ok(doc) => Ok(Some(doc)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DocumentError::from(e)),
    }
}

/// Get the extracted content of a document.
pub fn get_document_content(conn: &Connection, document_id: &str) -> Result<Option<String>, DocumentError> {
    let mut stmt = conn.prepare(
        "SELECT content FROM document_content WHERE document_id = ?1"
    )?;

    let result = stmt.query_row(params![document_id], |row| row.get(0));

    match result {
        Ok(content) => Ok(Some(content)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DocumentError::from(e)),
    }
}

/// Delete a document and its content.
pub fn delete_document(conn: &Connection, id: &str) -> Result<bool, DocumentError> {
    // Content is deleted automatically via CASCADE
    let rows = conn.execute("DELETE FROM documents WHERE id = ?1", params![id])?;
    Ok(rows > 0)
}

/// Helper to parse datetime strings.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_document_type_from_extension() {
        assert_eq!(DocumentType::from_extension("pdf"), Some(DocumentType::Pdf));
        assert_eq!(DocumentType::from_extension("PDF"), Some(DocumentType::Pdf));
        assert_eq!(DocumentType::from_extension("txt"), Some(DocumentType::Txt));
        assert_eq!(DocumentType::from_extension("md"), Some(DocumentType::Md));
        assert_eq!(DocumentType::from_extension("markdown"), Some(DocumentType::Md));
        assert_eq!(DocumentType::from_extension("doc"), None);
    }

    #[test]
    fn test_save_and_retrieve_document() {
        let conn = Connection::open_in_memory().unwrap();
        init_documents_table(&conn).unwrap();

        let doc = Document {
            id: "test-1".to_string(),
            name: "test.txt".to_string(),
            doc_type: DocumentType::Txt,
            size: 1234,
            uploaded_at: Utc::now(),
            path: "/tmp/test.txt".to_string(),
        };

        save_document(&conn, &doc).unwrap();
        save_document_content(&conn, "test-1", "Hello, world!").unwrap();

        let docs = get_all_documents(&conn).unwrap();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].name, "test.txt");

        let content = get_document_content(&conn, "test-1").unwrap();
        assert_eq!(content, Some("Hello, world!".to_string()));
    }
}
