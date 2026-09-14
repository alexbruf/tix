//! In-memory host for command tests and golden files (TIX-31).

use crate::host::{Host, PromptKind};
use crate::storage::{IoError, MemStorage, Storage};
use std::collections::VecDeque;

/// Default clock for tests: 2026-09-14T15:02:11Z.
pub const TEST_NOW: u64 = 1_789_398_131;

/// A recorded storage call, for host-contract assertions (TIX-31.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Call {
    Read(String),
    Write(String),
    ListDir(String),
    MkdirAll(String),
    Exists(String),
}

#[derive(Debug)]
pub struct MemHost {
    pub fs: MemStorage,
    pub stdout: String,
    pub stderr: String,
    /// Answers returned by successive `prompt` calls; `None` when exhausted.
    pub answers: VecDeque<Option<String>>,
    /// `(label, kind, options)` of every prompt.
    pub prompts: Vec<(String, String, Vec<String>)>,
    pub now: u64,
    /// Next byte returned by `random_bytes` (deterministic ULIDs).
    pub next_byte: u8,
    pub calls: std::cell::RefCell<Vec<Call>>,
}

impl Default for MemHost {
    fn default() -> Self {
        MemHost {
            fs: MemStorage::new(),
            stdout: String::new(),
            stderr: String::new(),
            answers: VecDeque::new(),
            prompts: Vec::new(),
            now: TEST_NOW,
            next_byte: 0,
            calls: std::cell::RefCell::new(Vec::new()),
        }
    }
}

/// Result of one `tix` invocation against a [`MemHost`].
#[derive(Debug)]
pub struct Outcome {
    pub code: u32,
    pub stdout: String,
    pub stderr: String,
}

impl MemHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs `tix <args>` in `cwd`, returning the exit code and the output it produced.
    pub fn run(&mut self, cwd: &str, args: &[&str]) -> Outcome {
        self.stdout.clear();
        self.stderr.clear();
        self.calls.borrow_mut().clear();
        let argv: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let code = crate::cli::run(self, &argv, cwd);
        Outcome {
            code,
            stdout: self.stdout.clone(),
            stderr: self.stderr.clone(),
        }
    }

    pub fn file(&self, path: &str) -> Option<String> {
        self.fs
            .read(path)
            .ok()
            .flatten()
            .map(|b| String::from_utf8(b).expect("utf-8"))
    }

    pub fn put(&mut self, path: &str, text: &str) {
        self.fs.write(path, text.as_bytes()).expect("mem write");
    }

    pub fn writes(&self) -> Vec<String> {
        self.calls
            .borrow()
            .iter()
            .filter_map(|c| match c {
                Call::Write(p) | Call::MkdirAll(p) => Some(p.clone()),
                _ => None,
            })
            .collect()
    }
}

impl Storage for MemHost {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        self.calls.borrow_mut().push(Call::Read(path.to_string()));
        self.fs.read(path)
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        self.calls.borrow_mut().push(Call::Write(path.to_string()));
        self.fs.write(path, bytes)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        self.calls
            .borrow_mut()
            .push(Call::ListDir(path.to_string()));
        self.fs.list_dir(path)
    }

    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        self.calls
            .borrow_mut()
            .push(Call::MkdirAll(path.to_string()));
        self.fs.mkdir_all(path)
    }

    fn exists(&self, path: &str) -> Result<bool, IoError> {
        self.calls.borrow_mut().push(Call::Exists(path.to_string()));
        self.fs.exists(path)
    }
}

impl Host for MemHost {
    fn prompt(&mut self, label: &str, kind: PromptKind, options: &[String]) -> Option<String> {
        self.prompts.push((
            label.to_string(),
            kind.as_str().to_string(),
            options.to_vec(),
        ));
        self.answers.pop_front().flatten()
    }

    fn stdout(&mut self, text: &str) {
        self.stdout.push_str(text);
    }

    fn stderr(&mut self, text: &str) {
        self.stderr.push_str(text);
    }

    fn now_unix(&mut self) -> u64 {
        self.now
    }

    fn random_bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n)
            .map(|_| {
                let b = self.next_byte;
                self.next_byte = self.next_byte.wrapping_add(1);
                b
            })
            .collect()
    }
}
