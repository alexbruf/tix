//! The host interface (TIX-4) as a trait, and the per-command context that
//! gives commands workspace-relative storage (TIX-3) plus output and prompts.

use crate::storage::{join_path, IoError, Storage};

/// `kind` argument of the `prompt` import.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PromptKind {
    String,
    Enum,
    Date,
    List,
}

impl PromptKind {
    pub fn as_str(self) -> &'static str {
        match self {
            PromptKind::String => "string",
            PromptKind::Enum => "enum",
            PromptKind::Date => "date",
            PromptKind::List => "list",
        }
    }
}

/// Everything the wasm module imports from its host (TIX-4). Storage paths
/// given to a `Host` are absolute; commands see them through [`Ctx`].
pub trait Host: Storage {
    fn prompt(&mut self, label: &str, kind: PromptKind, options: &[String]) -> Option<String>;
    fn stdout(&mut self, text: &str);
    fn stderr(&mut self, text: &str);
    fn now_unix(&mut self) -> u64;
    fn random_bytes(&mut self, n: usize) -> Vec<u8>;
}

/// Exit codes from the Commands section of the spec.
pub const EXIT_OK: u32 = 0;
pub const EXIT_VALIDATION: u32 = 1;
pub const EXIT_USAGE: u32 = 2;
pub const EXIT_IO: u32 = 3;

/// A command failure: `message` goes to stderr, `code` is the exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub code: u32,
    pub message: String,
}

impl Failure {
    pub fn validation(message: impl Into<String>) -> Self {
        Failure {
            code: EXIT_VALIDATION,
            message: message.into(),
        }
    }

    pub fn usage(message: impl Into<String>) -> Self {
        Failure {
            code: EXIT_USAGE,
            message: message.into(),
        }
    }
}

impl From<IoError> for Failure {
    fn from(e: IoError) -> Self {
        Failure {
            code: EXIT_IO,
            message: e.to_string(),
        }
    }
}

pub type CmdResult = Result<(), Failure>;

/// Wraps a host: storage, clock and randomness pass through; output is
/// captured into strings and prompts answer nothing. Used by `tix mcp` and
/// `tix-tui` to run a command in-process and read back its stdout/stderr
/// instead of the process's own.
pub struct CaptureHost<'a, H: Host> {
    inner: &'a mut H,
    pub out: String,
    pub err: String,
}

impl<'a, H: Host> CaptureHost<'a, H> {
    pub fn new(inner: &'a mut H) -> Self {
        CaptureHost {
            inner,
            out: String::new(),
            err: String::new(),
        }
    }
}

impl<H: Host> Storage for CaptureHost<'_, H> {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        self.inner.read(path)
    }
    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        self.inner.write(path, bytes)
    }
    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        self.inner.list_dir(path)
    }
    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        self.inner.mkdir_all(path)
    }
    fn exists(&self, path: &str) -> Result<bool, IoError> {
        self.inner.exists(path)
    }
}

impl<H: Host> Host for CaptureHost<'_, H> {
    fn prompt(&mut self, _: &str, _: PromptKind, _: &[String]) -> Option<String> {
        None
    }
    fn stdout(&mut self, text: &str) {
        self.out.push_str(text);
    }
    fn stderr(&mut self, text: &str) {
        self.err.push_str(text);
    }
    fn now_unix(&mut self) -> u64 {
        self.inner.now_unix()
    }
    fn random_bytes(&mut self, n: usize) -> Vec<u8> {
        self.inner.random_bytes(n)
    }
}

/// Per-command context. Implements [`Storage`] with paths relative to the
/// workspace root (TIX-3).
pub struct Ctx<'a, H: Host> {
    pub host: &'a mut H,
    /// Absolute workspace root, `/`-separated.
    pub root: String,
    pub json: bool,
    pub no_prompt: bool,
}

impl<H: Host> Ctx<'_, H> {
    fn abs(&self, rel: &str) -> Result<String, IoError> {
        join_path(&self.root, rel)
    }

    pub fn out(&mut self, text: &str) {
        self.host.stdout(text);
    }

    pub fn err(&mut self, text: &str) {
        self.host.stderr(text);
    }

    /// Prompts unless `--no-prompt` was given; `None` means no answer.
    pub fn prompt(&mut self, label: &str, kind: PromptKind, options: &[String]) -> Option<String> {
        if self.no_prompt {
            return None;
        }
        self.host.prompt(label, kind, options)
    }
}

impl<H: Host> Storage for Ctx<'_, H> {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        self.host.read(&self.abs(path)?)
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        let p = self.abs(path)?;
        self.host.write(&p, bytes)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        self.host.list_dir(&self.abs(path)?)
    }

    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        let p = self.abs(path)?;
        self.host.mkdir_all(&p)
    }

    fn exists(&self, path: &str) -> Result<bool, IoError> {
        self.host.exists(&self.abs(path)?)
    }
}
