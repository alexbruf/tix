//! Standalone `tix` binary for native targets and WASI (`wasm32-wasip1`): the
//! tix-io commands with the host implemented over `std`, plus `tix mcp` (the
//! shared MCP server over stdio). Not part of the npm package (TIX-29).

mod host;

use host::{normalize_dir, StdHost};
use std::io::{BufRead, Write};
use tix_io::mcp::{self, McpArgs};

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

/// `tix mcp`: one JSON-RPC message per stdin line, one response per stdout line.
fn serve_mcp(args: &[String]) -> i32 {
    let workspace = match mcp::parse_args(args, &cwd()) {
        McpArgs::Serve { workspace } => workspace,
        McpArgs::Help => {
            print!("{}", mcp::HELP);
            return 0;
        }
        McpArgs::Error(msg) => {
            eprintln!("{msg}");
            return 2;
        }
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { return 3 };
        if line.trim().is_empty() {
            continue;
        }
        if let Some(reply) = mcp::handle(&mut StdHost, &workspace, &line) {
            if writeln!(stdout, "{reply}")
                .and_then(|_| stdout.flush())
                .is_err()
            {
                return 3;
            }
        }
    }
    0
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    if argv.first().map(String::as_str) == Some("mcp") {
        std::process::exit(serve_mcp(&argv[1..]));
    }
    let code = tix_io::run(&mut StdHost, &argv, &cwd());
    let _ = std::io::stdout().flush();
    std::process::exit(code as i32);
}
