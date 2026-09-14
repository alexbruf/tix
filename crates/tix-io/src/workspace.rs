//! Workspace discovery (TIX-6).
//!
//! The Node host resolves `cwd` to an absolute, `/`-separated path before
//! calling into wasm (a Windows drive letter such as `C:/` is passed through
//! as-is). Commands address files relative to the workspace root this module
//! returns, via [`ws_path`].

use crate::storage::{join_path, IoError, Storage};

/// Splits a leading Windows drive letter (`C:`) off `path`, if present.
fn split_drive(path: &str) -> (&str, &str) {
    let bytes = path.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        path.split_at(2)
    } else {
        ("", path)
    }
}

/// Walks up from `cwd` looking for `tix.yaml`, returning the directory that
/// has it. `None` when no ancestor (up to the filesystem root) has one.
pub fn find_workspace<S: Storage>(storage: &S, cwd: &str) -> Result<Option<String>, IoError> {
    let (drive, rest) = split_drive(cwd);
    let mut segments: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
    loop {
        let dir = if segments.is_empty() {
            format!("{drive}/")
        } else {
            format!("{drive}/{}", segments.join("/"))
        };
        let candidate = join_path(&dir, "tix.yaml")?;
        if storage.exists(&candidate)? {
            return Ok(Some(dir));
        }
        if segments.is_empty() {
            return Ok(None);
        }
        segments.pop();
    }
}

/// Joins a workspace-relative path onto the workspace root.
pub fn ws_path(root: &str, rel: &str) -> Result<String, IoError> {
    join_path(root, rel)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::MemStorage;

    #[test]
    fn tix_6_find_workspace_walks_up() {
        let mut s = MemStorage::new();
        s.write("/proj/tix.yaml", b"version: 1").unwrap();
        let found = find_workspace(&s, "/proj/tickets/ABCD1234").unwrap();
        assert_eq!(found, Some("/proj".to_string()));
    }

    #[test]
    fn tix_6_find_workspace_at_cwd_itself() {
        let mut s = MemStorage::new();
        s.write("/proj/tix.yaml", b"version: 1").unwrap();
        assert_eq!(
            find_workspace(&s, "/proj").unwrap(),
            Some("/proj".to_string())
        );
    }

    #[test]
    fn tix_6_find_workspace_none_found() {
        let s = MemStorage::new();
        assert_eq!(find_workspace(&s, "/a/b/c").unwrap(), None);
    }

    #[test]
    fn tix_6_find_workspace_stops_at_windows_drive_root() {
        let s = MemStorage::new();
        assert_eq!(find_workspace(&s, "C:/Users/alex/proj").unwrap(), None);
    }

    #[test]
    fn tix_6_find_workspace_windows_drive() {
        let mut s = MemStorage::new();
        s.write("C:/proj/tix.yaml", b"version: 1").unwrap();
        assert_eq!(
            find_workspace(&s, "C:/proj/tickets/X").unwrap(),
            Some("C:/proj".to_string())
        );
    }

    #[test]
    fn tix_6_ws_path_joins_relative_to_root() {
        assert_eq!(
            ws_path("/proj", "tickets/ID/ticket.md").unwrap(),
            "/proj/tickets/ID/ticket.md"
        );
    }
}
