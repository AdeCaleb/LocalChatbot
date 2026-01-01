//! Database module for chat history persistence.
//!
//! This module demonstrates several important Rust patterns:
//! - Struct definitions with derived traits
//! - Error handling with Result<T, E>
//! - SQLite integration using rusqlite
//! - Serde serialization for Tauri IPC

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Represents a chat conversation.
///
/// The `#[derive(...)]` attribute auto-generates trait implementations:
/// - `Debug`: Allows printing with {:?}
/// - `Serialize/Deserialize`: Converts to/from JSON for Tauri IPC
/// - `Clone`: Allows creating copies of the struct
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Represents a single message in a chat.
///
/// Note: `sources` stores JSON as a string in SQLite.
/// SQLite doesn't have a native JSON type, so we serialize DocumentSource[]
/// to a JSON string when storing and deserialize when reading.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: String,
    pub chat_id: String,
    pub role: String, // "user" or "assistant"
    pub content: String,
    pub timestamp: DateTime<Utc>,
    pub sources: Option<String>, // JSON string of DocumentSource[]
}

/// A chat with all its messages - used when loading a full conversation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatWithMessages {
    pub id: String,
    pub title: String,
    pub messages: Vec<Message>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Database wrapper that manages SQLite connection and operations.
///
/// In Rust, we often wrap external resources in our own struct to:
/// 1. Provide a cleaner API tailored to our needs
/// 2. Add domain-specific methods
/// 3. Control access and ensure proper resource management
pub struct Database {
    /// The SQLite connection - public so document commands can access it
    pub conn: Connection,
}

impl Database {
    /// Creates a new Database, initializing the schema if needed.
    ///
    /// The `pub fn new` pattern is Rust's convention for constructors.
    /// Unlike languages with `new` keywords, Rust constructors are just
    /// regular associated functions that return Self.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        // Open or create the SQLite database file
        let conn = Connection::open(path)?;

        // Create a new Database instance
        let db = Database { conn };

        // Initialize tables - the `?` operator propagates errors
        // If init_schema() returns Err, this function returns early with that error
        db.init_schema()?;

        // Initialize document tables
        crate::documents::init_documents_table(&db.conn)?;

        Ok(db)
    }

    /// Initializes the database schema.
    ///
    /// SQLite's `IF NOT EXISTS` means this is safe to call multiple times.
    /// On first run, tables are created. On subsequent runs, it's a no-op.
    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        // Chats table - stores conversation metadata
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS chats (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )",
            [],
        )?;

        // Messages table - stores individual messages
        // FOREIGN KEY ensures referential integrity with CASCADE delete
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                chat_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                sources TEXT,
                FOREIGN KEY (chat_id) REFERENCES chats(id) ON DELETE CASCADE
            )",
            [],
        )?;

        // Enable foreign key enforcement (SQLite has it off by default)
        self.conn.execute("PRAGMA foreign_keys = ON", [])?;

        // Create index for faster message lookups by chat_id
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_messages_chat_id ON messages(chat_id)",
            [],
        )?;

        Ok(())
    }

    /// Creates a new chat conversation.
    ///
    /// Returns the created Chat struct on success.
    pub fn create_chat(&self, id: &str, title: &str) -> Result<Chat, rusqlite::Error> {
        let now = Utc::now();

        // `params!` macro creates a parameter array for safe SQL binding
        // This prevents SQL injection - NEVER concatenate user input into SQL strings!
        self.conn.execute(
            "INSERT INTO chats (id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)",
            params![id, title, now.to_rfc3339(), now.to_rfc3339()],
        )?;

        Ok(Chat {
            id: id.to_string(),
            title: title.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Retrieves all chats, ordered by most recently updated.
    ///
    /// This demonstrates Rust iterators and collecting results.
    pub fn get_all_chats(&self) -> Result<Vec<Chat>, rusqlite::Error> {
        let mut stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at FROM chats ORDER BY updated_at DESC"
        )?;

        // `query_map` returns an iterator over rows
        // We map each row to a Chat struct, then collect into a Vec
        let chats = stmt.query_map([], |row| {
            Ok(Chat {
                id: row.get(0)?,
                title: row.get(1)?,
                // Parse ISO 8601 datetime strings back to DateTime<Utc>
                created_at: parse_datetime(&row.get::<_, String>(2)?),
                updated_at: parse_datetime(&row.get::<_, String>(3)?),
            })
        })?;

        // Collect results, propagating any errors
        // The turbofish `::<Vec<_>>` tells Rust what type to collect into
        chats.collect::<Result<Vec<_>, _>>()
    }

    /// Gets a single chat with all its messages.
    pub fn get_chat(&self, chat_id: &str) -> Result<Option<ChatWithMessages>, rusqlite::Error> {
        // First, get the chat metadata
        let mut chat_stmt = self.conn.prepare(
            "SELECT id, title, created_at, updated_at FROM chats WHERE id = ?1"
        )?;

        let chat = chat_stmt.query_row(params![chat_id], |row| {
            Ok(Chat {
                id: row.get(0)?,
                title: row.get(1)?,
                created_at: parse_datetime(&row.get::<_, String>(2)?),
                updated_at: parse_datetime(&row.get::<_, String>(3)?),
            })
        });

        // Handle the case where chat doesn't exist
        let chat = match chat {
            Ok(c) => c,
            Err(rusqlite::Error::QueryReturnedNoRows) => return Ok(None),
            Err(e) => return Err(e),
        };

        // Then get all messages for this chat
        let mut msg_stmt = self.conn.prepare(
            "SELECT id, chat_id, role, content, timestamp, sources
             FROM messages WHERE chat_id = ?1 ORDER BY timestamp ASC"
        )?;

        let messages = msg_stmt.query_map(params![chat_id], |row| {
            Ok(Message {
                id: row.get(0)?,
                chat_id: row.get(1)?,
                role: row.get(2)?,
                content: row.get(3)?,
                timestamp: parse_datetime(&row.get::<_, String>(4)?),
                sources: row.get(5)?,
            })
        })?;

        let messages: Vec<Message> = messages.collect::<Result<Vec<_>, _>>()?;

        Ok(Some(ChatWithMessages {
            id: chat.id,
            title: chat.title,
            messages,
            created_at: chat.created_at,
            updated_at: chat.updated_at,
        }))
    }

    /// Deletes a chat and all its messages (via CASCADE).
    pub fn delete_chat(&self, chat_id: &str) -> Result<bool, rusqlite::Error> {
        let rows_affected = self.conn.execute(
            "DELETE FROM chats WHERE id = ?1",
            params![chat_id],
        )?;

        // Return true if a chat was actually deleted
        Ok(rows_affected > 0)
    }

    /// Adds a message to a chat.
    pub fn add_message(&self, message: &Message) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "INSERT INTO messages (id, chat_id, role, content, timestamp, sources)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                message.id,
                message.chat_id,
                message.role,
                message.content,
                message.timestamp.to_rfc3339(),
                message.sources,
            ],
        )?;

        // Update the chat's updated_at timestamp
        self.conn.execute(
            "UPDATE chats SET updated_at = ?1 WHERE id = ?2",
            params![Utc::now().to_rfc3339(), message.chat_id],
        )?;

        Ok(())
    }

    /// Updates a chat's title.
    pub fn update_chat_title(&self, chat_id: &str, title: &str) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "UPDATE chats SET title = ?1, updated_at = ?2 WHERE id = ?3",
            params![title, Utc::now().to_rfc3339(), chat_id],
        )?;
        Ok(())
    }
}

/// Helper function to parse datetime strings.
///
/// Falls back to current time if parsing fails - this is a pragmatic choice
/// to prevent crashes on corrupted data. In production, you might want
/// to handle this differently based on your requirements.
fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests run with `cargo test` in the src-tauri directory.
    /// The `#[test]` attribute marks this as a test function.
    #[test]
    fn test_create_and_retrieve_chat() {
        // ":memory:" creates an in-memory database for testing
        let db = Database::new(":memory:").unwrap();

        let chat = db.create_chat("test-1", "Test Chat").unwrap();
        assert_eq!(chat.id, "test-1");
        assert_eq!(chat.title, "Test Chat");

        let chats = db.get_all_chats().unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title, "Test Chat");
    }

    #[test]
    fn test_add_message() {
        let db = Database::new(":memory:").unwrap();
        db.create_chat("chat-1", "Test").unwrap();

        let msg = Message {
            id: "msg-1".to_string(),
            chat_id: "chat-1".to_string(),
            role: "user".to_string(),
            content: "Hello!".to_string(),
            timestamp: Utc::now(),
            sources: None,
        };

        db.add_message(&msg).unwrap();

        let chat = db.get_chat("chat-1").unwrap().unwrap();
        assert_eq!(chat.messages.len(), 1);
        assert_eq!(chat.messages[0].content, "Hello!");
    }

    #[test]
    fn test_delete_chat_cascades() {
        let db = Database::new(":memory:").unwrap();
        db.create_chat("chat-1", "Test").unwrap();

        let msg = Message {
            id: "msg-1".to_string(),
            chat_id: "chat-1".to_string(),
            role: "user".to_string(),
            content: "Hello!".to_string(),
            timestamp: Utc::now(),
            sources: None,
        };
        db.add_message(&msg).unwrap();

        // Delete should cascade to messages
        db.delete_chat("chat-1").unwrap();

        let chat = db.get_chat("chat-1").unwrap();
        assert!(chat.is_none());
    }
}
