//! Standalone `tix` binary for native targets and WASI (`wasm32-wasip1`): the
//! tix-io commands with the host implemented over `std`, plus `tix mcp` (the
//! shared MCP server over stdio). Not part of the npm package (TIX-29).

use std::io::{BufRead, Write};
use tix_cli::host::{cwd, StdHost};
use tix_io::mcp::{self, McpArgs};

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
