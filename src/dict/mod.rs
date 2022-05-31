use std::env;
use std::collections::HashSet;
use tokio::io::AsyncReadExt;
use std::path::{Path, PathBuf};

pub const DICT_VARNAME: &str = "WORDCLASH_DICTIONARY";
pub const DICT_FILENAME: &str = "dictionary.json";

// Get dictionary path from environment variables or executable path.
pub fn get_dict_path() -> PathBuf {
    env::var(DICT_VARNAME)
        .map(|dict| PathBuf::from(dict))
        .or_else(|_| std::env::current_exe()
            .map(|mut p| {
                p.pop();
                p.push(DICT_FILENAME);
                p
            }))
        .expect("Dictionary not found")
}

// Load dictionary from specified path.
pub async fn load_dictionary_from(path: &Path) -> HashSet<String> {
    let mut file = tokio::fs::File::open(path)
        .await
        .expect("Failed to open dictionary");
    let mut buf = Vec::new();
    file.read_to_end(&mut buf).await
        .expect("Failed to read dictionary");
    serde_json::from_slice(&buf)
        .expect("Failed to deserialize dictionary")
}

// Load dictionary from default path.
pub async fn load_dictionary() -> HashSet<String> {
    let path = get_dict_path();
    load_dictionary_from(&path).await
}

pub mod wordmatch;