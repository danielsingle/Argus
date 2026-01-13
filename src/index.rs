//! Index module for caching extracted text from files.
//!
//! This module provides functionality to save and load an index of extracted text,
//! allowing subsequent searches to skip expensive text extraction for unchanged files.

use crate::types::FileType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{BufReader, BufWriter};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Current index format version. Increment when making breaking changes.
const INDEX_VERSION: u32 = 1;

/// A single entry in the index representing a cached file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexEntry {
    /// Path to the file (relative to index directory for portability).
    pub path: PathBuf,
    /// Type of the file.
    pub file_type: FileType,
    /// Extracted text content from the file.
    pub extracted_text: String,
    /// File modification timestamp (Unix timestamp in seconds).
    pub modified_timestamp: u64,
    /// File size in bytes.
    pub file_size: u64,
}

impl IndexEntry {
    /// Create a new index entry.
    pub fn new(
        path: PathBuf,
        file_type: FileType,
        extracted_text: String,
        modified_timestamp: u64,
        file_size: u64,
    ) -> Self {
        Self {
            path,
            file_type,
            extracted_text,
            modified_timestamp,
            file_size,
        }
    }

    /// Check if this entry is stale (file has been modified since indexing).
    pub fn is_stale(&self, current_modified: u64, current_size: u64) -> bool {
        self.modified_timestamp != current_modified || self.file_size != current_size
    }
}

/// The main index structure containing all cached file entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Index {
    /// Index format version for compatibility checking.
    pub version: u32,
    /// The directory this index covers.
    pub directory: PathBuf,
    /// When this index was created (Unix timestamp).
    pub created_at: u64,
    /// When this index was last updated (Unix timestamp).
    pub updated_at: u64,
    /// Map of file paths to their cached entries.
    pub entries: HashMap<PathBuf, IndexEntry>,
}

impl Index {
    /// Create a new empty index for the given directory.
    pub fn new(directory: PathBuf) -> Self {
        let now = current_timestamp();
        Self {
            version: INDEX_VERSION,
            directory,
            created_at: now,
            updated_at: now,
            entries: HashMap::new(),
        }
    }

    /// Load an index from a file.
    pub fn load(path: &PathBuf) -> Result<Self, IndexError> {
        if !path.exists() {
            return Err(IndexError::NotFound(path.clone()));
        }

        let file = File::open(path).map_err(|e| IndexError::IoError(e.to_string()))?;
        let reader = BufReader::new(file);
        let index: Index =
            serde_json::from_reader(reader).map_err(|e| IndexError::ParseError(e.to_string()))?;

        // Check version compatibility
        if index.version != INDEX_VERSION {
            return Err(IndexError::VersionMismatch {
                expected: INDEX_VERSION,
                found: index.version,
            });
        }

        Ok(index)
    }

    /// Save the index to a file.
    pub fn save(&mut self, path: &PathBuf) -> Result<(), IndexError> {
        // Update the timestamp
        self.updated_at = current_timestamp();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| IndexError::IoError(e.to_string()))?;
        }

        let file = File::create(path).map_err(|e| IndexError::IoError(e.to_string()))?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self)
            .map_err(|e| IndexError::IoError(e.to_string()))?;

        Ok(())
    }

    /// Get a cached entry if it exists and is not stale.
    pub fn get_valid_entry(&self, path: &PathBuf) -> Option<&IndexEntry> {
        let entry = self.entries.get(path)?;

        // Check if the file still exists and hasn't been modified
        let metadata = path.metadata().ok()?;
        let current_modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let current_size = metadata.len();

        if entry.is_stale(current_modified, current_size) {
            None
        } else {
            Some(entry)
        }
    }

    /// Add or update an entry in the index.
    pub fn upsert_entry(&mut self, entry: IndexEntry) {
        self.entries.insert(entry.path.clone(), entry);
    }

    /// Remove entries for files that no longer exist.
    pub fn prune_missing(&mut self) {
        self.entries.retain(|path, _| path.exists());
    }

    /// Get the number of entries in the index.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the index is empty.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// Errors that can occur during index operations.
#[derive(Debug)]
pub enum IndexError {
    /// Index file not found.
    NotFound(PathBuf),
    /// IO error during read/write.
    IoError(String),
    /// Failed to parse index file.
    ParseError(String),
    /// Index version mismatch.
    VersionMismatch { expected: u32, found: u32 },
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexError::NotFound(path) => write!(f, "Index file not found: {}", path.display()),
            IndexError::IoError(msg) => write!(f, "IO error: {}", msg),
            IndexError::ParseError(msg) => write!(f, "Failed to parse index: {}", msg),
            IndexError::VersionMismatch { expected, found } => {
                write!(
                    f,
                    "Index version mismatch: expected {}, found {}",
                    expected, found
                )
            }
        }
    }
}

impl std::error::Error for IndexError {}

/// Get the current Unix timestamp in seconds.
pub fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Get the modification timestamp of a file.
pub fn get_file_timestamp(path: &Path) -> Option<u64> {
    path.metadata()
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_save_index() {
        let dir = tempdir().unwrap();
        let index_path = dir.path().join(".argus_index.json");

        let mut index = Index::new(dir.path().to_path_buf());
        index.upsert_entry(IndexEntry::new(
            dir.path().join("test.txt"),
            FileType::Text,
            "Hello World".to_string(),
            12345,
            11,
        ));

        assert!(index.save(&index_path).is_ok());
        assert!(index_path.exists());
    }

    #[test]
    fn test_load_index() {
        let dir = tempdir().unwrap();
        let index_path = dir.path().join(".argus_index.json");

        // Create and save an index
        let mut index = Index::new(dir.path().to_path_buf());
        index.upsert_entry(IndexEntry::new(
            dir.path().join("test.txt"),
            FileType::Text,
            "Hello World".to_string(),
            12345,
            11,
        ));
        index.save(&index_path).unwrap();

        // Load it back
        let loaded = Index::load(&index_path).unwrap();
        assert_eq!(loaded.entries.len(), 1);
        assert_eq!(loaded.version, INDEX_VERSION);
    }

    #[test]
    fn test_stale_entry_detection() {
        let entry = IndexEntry::new(
            PathBuf::from("test.txt"),
            FileType::Text,
            "content".to_string(),
            1000,
            100,
        );

        // Same timestamp and size - not stale
        assert!(!entry.is_stale(1000, 100));

        // Different timestamp - stale
        assert!(entry.is_stale(1001, 100));

        // Different size - stale
        assert!(entry.is_stale(1000, 101));
    }

    #[test]
    fn test_get_valid_entry() {
        let dir = tempdir().unwrap();
        let test_file = dir.path().join("test.txt");
        fs::write(&test_file, "Hello").unwrap();

        let metadata = test_file.metadata().unwrap();
        let modified = metadata
            .modified()
            .unwrap()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let size = metadata.len();

        let mut index = Index::new(dir.path().to_path_buf());
        index.upsert_entry(IndexEntry::new(
            test_file.clone(),
            FileType::Text,
            "Hello".to_string(),
            modified,
            size,
        ));

        // Entry should be valid
        assert!(index.get_valid_entry(&test_file).is_some());

        // Modify the file
        fs::write(&test_file, "Hello World - modified").unwrap();

        // Entry should now be stale
        assert!(index.get_valid_entry(&test_file).is_none());
    }
}
