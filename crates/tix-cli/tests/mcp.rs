//! `tix mcp` over real stdio: initialize, list tools, and drive a board.
#![cfg(not(target_os = "wasi"))]

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

impl Session {
    fn start(workspace: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_tix"))
            .args(["mcp", "--workspace"])
            .arg(workspace)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn tix mcp");
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut s = Session {
            child,
            stdin,
            stdout,
            next_id: 0,
        };
        let init = s.request(
            "initialize",
            json!({"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "test", "version": "1"}}),
        );
        assert_eq!(init["result"]["serverInfo"]["name"], "tix");
        s.send(json!({"jsonrpc": "2.0", "method": "notifications/initialized"}));
        s
    }

    fn send(&mut self, msg: Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        self.send(json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}));
        loop {
            let mut line = String::new();
            assert!(
                self.stdout.read_line(&mut line).unwrap() > 0,
                "server closed"
            );
            let msg: Value = serde_json::from_str(&line).unwrap();
            if msg["id"] == json!(id) {
                return msg;
            }
        }
    }

    /// Calls a tool, returning (is_error, text).
    fn call(&mut self, name: &str, args: Value) -> (bool, String) {
        let r = self.request("tools/call", json!({"name": name, "arguments": args}));
        let res = &r["result"];
        let text = res["content"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["text"].as_str().unwrap())
            .collect();
        (res["isError"].as_bool().unwrap_or(false), text)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[test]
fn mcp_lists_tools_and_drives_a_board() {
    let dir = std::env::temp_dir().join(format!("tix-mcp-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut s = Session::start(&dir);

    let tools = s.request("tools/list", json!({}));
    let mut names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "tix_attach",
            "tix_board",
            "tix_check",
            "tix_detach",
            "tix_help",
            "tix_init",
            "tix_ls",
            "tix_mv",
            "tix_new",
            "tix_path",
            "tix_set",
            "tix_show"
        ]
    );

    let (err, text) = s.call("tix_ls", json!({}));
    assert!(
        err && text.starts_with("exit 2") && text.contains("no tix.yaml found"),
        "{text}"
    );

    assert!(!s.call("tix_init", json!({})).0);
    let (_, help) = s.call("tix_help", json!({"command": "new"}));
    assert!(
        help.contains("--type <article|landing_page|linkedin|other>"),
        "{help}"
    );

    let (err, text) = s.call(
        "tix_new",
        json!({"title": "Q4 article", "fields": {"client": "acme", "type": "article"}}),
    );
    assert!(!err, "{text}");
    let id = serde_json::from_str::<Value>(&text).unwrap()["id"]
        .as_str()
        .unwrap()
        .to_string();

    let (err, text) = s.call(
        "tix_new",
        json!({"title": "bad", "fields": {"client": "acme", "type": "nope"}}),
    );
    assert!(
        err && text.starts_with("exit 1") && text.contains("rule 4"),
        "{text}"
    );

    let (err, text) = s.call(
        "tix_set",
        json!({"id": id, "values": {"status": "in_progress", "owner": "sam"}}),
    );
    assert!(!err, "{text}");
    let t: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        (t["status"].as_str(), t["owner"].as_str()),
        (Some("in_progress"), Some("sam"))
    );

    assert!(
        !s.call("tix_attach", json!({"id": id, "ref": "./outline.md"}))
            .0
    );
    let (_, shown) = s.call("tix_show", json!({"id": &id[..8]}));
    assert!(shown.contains("\"label\":\"outline.md\""), "{shown}");

    // Rename a status in the schema: check reports the ticket (error result carrying the
    // JSON problems), and moving it to the new status repairs it.
    let yaml = std::fs::read_to_string(dir.join("tix.yaml"))
        .unwrap()
        .replace("name: in_progress", "name: doing");
    std::fs::write(dir.join("tix.yaml"), yaml).unwrap();
    let (err, text) = s.call("tix_check", json!({}));
    assert!(
        err && text.contains("\"problems\"") && text.contains("rule 2"),
        "{text}"
    );
    assert!(!s.call("tix_mv", json!({"id": id, "status": "doing"})).0);
    let (err, text) = s.call("tix_check", json!({}));
    assert!(!err && text.contains("\"ok\":true"), "{text}");

    let _ = std::fs::remove_dir_all(&dir);
}
