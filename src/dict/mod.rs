use std::collections::HashMap;
use rand::prelude::*;
use indexmap::IndexSet;
use std::env;
use std::path::{Path, PathBuf};

pub const DICT_VARNAME: &str = "WORDCLASH_DICTIONARY";
pub const DICT_FILENAME: &str = "dictionary.json";

pub type DictSet = IndexSet<String>;

pub struct Dictionary {
    data: HashMap<usize, DictSet>,
}

impl Dictionary {
    pub fn new(src: DictSet) -> Dictionary {
        let mut lengthmap: HashMap<usize, Vec<String>> = HashMap::new();
        for s in src.into_iter() {
            lengthmap.entry(s.len()).or_default().push(s);
        }

        Dictionary {
            data: lengthmap
                .into_iter()
                .map(|(k, v)| (k, v.into_iter().collect()))
                .collect()
        }
    }

    pub fn contains(&self, word: &str) -> bool {
        self.data
            .get(&word.len())
            .map_or(false, |set| set.contains(word))
    }
    
    pub fn random_with_len(&self, len: usize) -> Option<&String> {
        let set = match self.data.get(&len) {
            Some(s) => s, None => return None
        };
        let index = rand::distributions::Uniform::new(0, set.len())
            .sample(&mut thread_rng());
        set.get_index(index)
    }
}

// Get dictionary path from environment variables or executable path.
pub fn get_dict_path() -> PathBuf {
    env::var(DICT_VARNAME)
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::current_exe().map(|mut p| {
                p.pop();
                p.push(DICT_FILENAME);
                p
            })
        })
        .expect("Dictionary not found")
}

// Load dictionary as raw set of words from specified path.
pub fn load_dictset_from(path: &Path) -> DictSet {
    let mut file = std::fs::File::open(path)
        .expect("Failed to open dictionary");
    let mut buf = Vec::new();
    use std::io::Read;
    file.read_to_end(&mut buf)
        .expect("Failed to read dictionary");
    serde_json::from_slice(&buf).expect("Failed to deserialize dictionary")
}

// Load dictionary.
pub fn load_dictionary() -> Dictionary {
    let path = get_dict_path();
    Dictionary::new(load_dictset_from(&path))
}

pub mod wordmatch;

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_contains() {
        let set = DictSet::from(["churro", "squat", "stance"]
            .map(|s| s.to_string()));
        let dict = Dictionary::new(set);
        assert!(dict.contains("churro"));
        assert!(!dict.contains("churros"));
        assert!(dict.contains("squat"));
        assert!(!dict.contains("stanza"));
        assert!(dict.contains("stance"));
    }

    #[test]
    fn test_random() {
        let set = DictSet::from(["churro", "squat", "running", "vision", "fall"]
            .map(|s| s.to_string()));
        let dict = Dictionary::new(set);

        let mut count_churro = 0_usize;
        let mut count_vision = 0_usize;
        let security = 256;
        for _ in 0..security {
            assert!(dict.random_with_len(5).map_or(false, |s| s == "squat"));
            assert!(dict.random_with_len(7).map_or(false, |s| s == "running"));
            assert!(dict.random_with_len(4).map_or(false, |s| s == "fall"));
            
            assert!(dict.random_with_len(6).map_or(false,
                |s| match s.as_str() {
                    "churro" => {count_churro += 1; true},
                    "vision" => {count_vision += 1; true},
                    _ => false,
                }));
        }

        // This might fail, but that's about as likely as guessing a ${security}-bit key in 2 tries
        assert!(count_churro > 0);
        assert!(count_vision > 0);
        assert!(count_churro == security-count_vision);
    }
}