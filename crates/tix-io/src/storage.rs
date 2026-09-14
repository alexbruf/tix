//! The `Storage` trait (TIX-3), a path-join helper, and an in-memory
//! implementation used by tests.

use std::collections::{BTreeMap, BTreeSet};

/// An I/O failure. Carries a human-readable message; callers that need to
/// distinguish cases should match on the message or wrap this type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IoError {
    pub message: String,
}

impl IoError {
    pub fn new(message: impl Into<String>) -> Self {
        IoError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for IoError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for IoError {}

/// All filesystem access goes through these five methods (TIX-3). Paths are
/// `/`-separated and never contain `..`.
pub trait Storage {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError>;
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError>;
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError>;
    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError>;
    fn exists(&self, path: &str) -> Result<bool, IoError>;
}

/// Joins `base` and `rel` with a single `/`, tolerating extra slashes on
/// either side, and rejects a joined path containing a `..` segment.
pub fn join_path(base: &str, rel: &str) -> Result<String, IoError> {
    let joined = if base.is_empty() {
        rel.to_string()
    } else if rel.is_empty() {
        base.to_string()
    } else {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            rel.trim_start_matches('/')
        )
    };
    if joined.split('/').any(|seg| seg == "..") {
        return Err(IoError::new(format!("path contains '..': {joined}")));
    }
    Ok(joined)
}

/// In-memory `Storage` for tests. Directories created via `mkdir_all` are
/// tracked explicitly so `list_dir`/`exists` see empty directories too.
#[derive(Default, Clone, Debug)]
pub struct MemStorage {
    files: BTreeMap<String, Vec<u8>>,
    dirs: BTreeSet<String>,
}

impl MemStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Storage for MemStorage {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        Ok(self.files.get(path).cloned())
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        if let Some(idx) = path.rfind('/') {
            self.mkdir_all(&path[..idx])?;
        }
        self.files.insert(path.to_string(), bytes.to_vec());
        Ok(())
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        let prefix = format!("{}/", path.trim_end_matches('/'));
        let mut names: BTreeSet<String> = BTreeSet::new();
        for p in self.files.keys().chain(self.dirs.iter()) {
            if let Some(rest) = p.strip_prefix(prefix.as_str()) {
                if let Some(name) = rest.split('/').next() {
                    if !name.is_empty() {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        Ok(names.into_iter().collect())
    }

    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        let absolute = path.starts_with('/');
        let mut acc = String::new();
        for seg in path.split('/') {
            if seg.is_empty() {
                continue;
            }
            if !acc.is_empty() || absolute {
                acc.push('/');
            }
            acc.push_str(seg);
            self.dirs.insert(acc.clone());
        }
        Ok(())
    }

    fn exists(&self, path: &str) -> Result<bool, IoError> {
        Ok(self.files.contains_key(path) || self.dirs.contains(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tix_3_paths_reject_dotdot() {
        assert!(join_path("tickets", "../tix.yaml").is_err());
        assert!(join_path("tickets/../ID", "ticket.md").is_err());
        assert!(join_path("tickets", "ID/ticket.md").is_ok());
    }

    #[test]
    fn tix_3_join_path_normalizes_slashes() {
        assert_eq!(join_path("root/", "/rel").unwrap(), "root/rel");
        assert_eq!(join_path("", "rel").unwrap(), "rel");
        assert_eq!(join_path("root", "").unwrap(), "root");
    }

    #[test]
    fn tix_3_mem_storage_round_trips() {
        let mut s = MemStorage::new();
        assert!(!s.exists("tickets").unwrap());
        s.write("tickets/ID1/ticket.md", b"hello").unwrap();
        assert_eq!(
            s.read("tickets/ID1/ticket.md").unwrap(),
            Some(b"hello".to_vec())
        );
        assert!(s.exists("tickets/ID1").unwrap());
        assert!(s.exists("tickets").unwrap());
        assert_eq!(s.read("tickets/ID1/missing").unwrap(), None);
    }

    #[test]
    fn tix_3_list_dir_returns_sorted_immediate_children() {
        let mut s = MemStorage::new();
        s.mkdir_all("tickets/B").unwrap();
        s.write("tickets/A/ticket.md", b"x").unwrap();
        s.write("tickets/A/nested/file.md", b"y").unwrap();
        assert_eq!(
            s.list_dir("tickets").unwrap(),
            vec!["A".to_string(), "B".to_string()]
        );
        assert_eq!(
            s.list_dir("tickets/A").unwrap(),
            vec!["nested".to_string(), "ticket.md".to_string()]
        );
    }
}
