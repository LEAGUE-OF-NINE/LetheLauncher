use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileManifest {
    pub scanned_folder: String,
    pub total_files: usize,
    pub total_size: u64,
    pub files: Vec<FileEntry>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub xxhash: String,
}
