//! Standalone `tix` binary for native targets and WASI (`wasm32-wasip1`): the
//! tix-io commands with the host implemented over `std`. Native builds also
//! serve the commands as MCP tools with `tix mcp`. Not part of the npm package
//! (TIX-29).

mod host;
#[cfg(not(target_os = "wasi"))]
mod mcp;

use host::{normalize_dir, StdHost};
use std::io::Write;

/// Absolute, `/`-separated working directory. `TIX_CWD` overrides it for WASI
/// runtimes whose guest cwd does not match the mounted directory.
fn cwd() -> String {
    let raw = std::env::var("TIX_CWD")
        .ok()
        .or_else(|| {
            std::env::current_dir()
                .ok()
                .map(|p| p.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "/".to_string());
    normalize_dir(&raw)
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    #[cfg(not(target_os = "wasi"))]
    if argv.first().map(String::as_str) == Some("mcp") {
        std::process::exit(mcp::main(&argv[1..], cwd()));
    }
    let code = tix_io::run(&mut StdHost::default(), &argv, &cwd());
    let _ = std::io::stdout().flush();
    std::process::exit(code as i32);
}
