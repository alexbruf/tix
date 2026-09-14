//! wasm-bindgen glue (TIX-4): imports the ten host functions from the
//! `tix_host` object the JavaScript host installs on `globalThis`, and exports
//! `run(argv, cwd) -> u32`.

use tix_io::host::{Host, PromptKind};
use tix_io::storage::{IoError, Storage};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = tix_host, catch)]
    fn fs_read(path: &str) -> Result<Option<Vec<u8>>, JsValue>;
    #[wasm_bindgen(js_namespace = tix_host, catch)]
    fn fs_write(path: &str, bytes: &[u8]) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = tix_host, catch)]
    fn fs_list_dir(path: &str) -> Result<Vec<String>, JsValue>;
    #[wasm_bindgen(js_namespace = tix_host, catch)]
    fn fs_mkdir_all(path: &str) -> Result<(), JsValue>;
    #[wasm_bindgen(js_namespace = tix_host, catch)]
    fn fs_exists(path: &str) -> Result<bool, JsValue>;
    #[wasm_bindgen(js_namespace = tix_host)]
    fn prompt(label: &str, kind: &str, options: Vec<String>) -> Option<String>;
    #[wasm_bindgen(js_namespace = tix_host)]
    fn stdout(text: &str);
    #[wasm_bindgen(js_namespace = tix_host)]
    fn stderr(text: &str);
    #[wasm_bindgen(js_namespace = tix_host)]
    fn now_unix() -> u64;
    #[wasm_bindgen(js_namespace = tix_host)]
    fn random_bytes(n: u32) -> Vec<u8>;
}

/// A thrown JavaScript error becomes an `IoError` (exit 3).
fn io<T>(r: Result<T, JsValue>, op: &str, path: &str) -> Result<T, IoError> {
    r.map_err(|e| {
        let detail = e.as_string().unwrap_or_else(|| format!("{e:?}"));
        IoError::new(format!("{op} {path}: {detail}"))
    })
}

struct JsHost;

impl Storage for JsHost {
    fn read(&self, path: &str) -> Result<Option<Vec<u8>>, IoError> {
        io(fs_read(path), "read", path)
    }

    fn write(&mut self, path: &str, bytes: &[u8]) -> Result<(), IoError> {
        io(fs_write(path, bytes), "write", path)
    }

    fn list_dir(&self, path: &str) -> Result<Vec<String>, IoError> {
        io(fs_list_dir(path), "list", path)
    }

    fn mkdir_all(&mut self, path: &str) -> Result<(), IoError> {
        io(fs_mkdir_all(path), "mkdir", path)
    }

    fn exists(&self, path: &str) -> Result<bool, IoError> {
        io(fs_exists(path), "stat", path)
    }
}

impl Host for JsHost {
    fn prompt(&mut self, label: &str, kind: PromptKind, options: &[String]) -> Option<String> {
        prompt(label, kind.as_str(), options.to_vec())
    }

    fn stdout(&mut self, text: &str) {
        stdout(text);
    }

    fn stderr(&mut self, text: &str) {
        stderr(text);
    }

    fn now_unix(&mut self) -> u64 {
        now_unix()
    }

    fn random_bytes(&mut self, n: usize) -> Vec<u8> {
        random_bytes(n as u32)
    }
}

/// Runs one `tix` invocation and returns the process exit code (TIX-4).
#[wasm_bindgen]
pub fn run(argv: Vec<String>, cwd: String) -> u32 {
    tix_io::run(&mut JsHost, &argv, &cwd)
}
