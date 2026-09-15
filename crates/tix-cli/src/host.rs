//! `Host` over `std`: the filesystem, stdin prompts, the clock and OS randomness.

use std::io::{BufRead, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use tix_io::host::{Host, PromptKind};
use tix_io::storage::{IoError, Storage};

pub struct StdHost;

fn io_err(op: &str, path: &str, e: std::io::Error) -> IoError {
    IoError::new(format!("{op} {path}: {e}"))
}

impl Storage for StdHost {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        match std::fs::read(path) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(io_err("read", path, e)),
        }
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        std::fs::write(path, bytes).map_err(|e| io_err("write", path, e))
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        let entries = std::fs::read_dir(path).map_err(|e| io_err("list", path, e))?;
        entries
            .map(|e| {
                e.map(|e| e.file_name().to_string_lossy().into_owned())
                    .map_err(|err| io_err("list", path, err))
            })
            .collect()
    }

    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        std::fs::create_dir_all(path).map_err(|e| io_err("mkdir", path, e))
    }

    fn exists(&self, path: &str) -> Result<bool, IoError> {
        std::fs::exists(path).map_err(|e| io_err("stat", path, e))
    }
}

impl Host for StdHost {
    fn prompt(&mut self, label: &str, kind: PromptKind, options: &[String]) -> Option<String> {
        let hint = match kind {
            _ if !options.is_empty() => format!(" [{}]", options.join("/")),
            PromptKind::Date => " (YYYY-MM-DD)".to_string(),
            PromptKind::List => " (comma-separated)".to_string(),
            _ => String::new(),
        };
        eprint!("{label}{hint}: ");
        let _ = std::io::stderr().flush();
        let mut line = String::new();
        match std::io::stdin().lock().read_line(&mut line) {
            Ok(0) | Err(_) => None,
            Ok(_) => Some(line.trim().to_string()),
        }
    }

    fn stdout(&mut self, text: &str) {
        print!("{text}");
    }

    fn stderr(&mut self, text: &str) {
        eprint!("{text}");
    }

    fn now_unix(&mut self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    fn random_bytes(&mut self, n: usize) -> Vec<u8> {
        let mut buf = vec![0u8; n];
        getrandom::fill(&mut buf).expect("system randomness unavailable");
        buf
    }
}

/// Normalizes a directory to the absolute, `/`-separated form tix-io expects.
pub fn normalize_dir(raw: &str) -> String {
    raw.replace('\\', "/")
}
