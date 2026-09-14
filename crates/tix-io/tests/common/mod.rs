//! Shared fixture workspace and golden-file assertions for command tests (TIX-31).
#![allow(dead_code)]

use std::path::PathBuf;
use tix_io::schema_yaml::DEFAULT_TIX_YAML;
use tix_io::testkit::{MemHost, Outcome};

pub const WS: &str = "/ws";

pub const T_BACKLOG: &str = "01K5AQ9Z3R7M8N2P4Q6S8T0V1W";
pub const T_PROGRESS: &str = "01K5B2C3D4E5F6G7H8J9K0M1N2";
pub const T_DONE: &str = "01K5C7Q8R9S0T1V2W3X4Y5Z6A7";
/// Valid ULID, but missing the required `client` field (invalid per TIX-11 rule 3).
pub const T_NO_CLIENT: &str = "01K5D1E2F3G4H5J6K7M8N9P0Q1";
/// Status `review` is not in the default schema (TIX-10 `?` column).
pub const T_UNKNOWN_STATUS: &str = "01K5E9F8G7H6J5K4M3N2P1Q0R9";

fn ticket(id: &str, title: &str, status: &str, created: &str, extra: &str, body: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nstatus: {status}\ncreated: {created}\nupdated: {created}\ndeliverables: []\n{extra}---\n{body}"
    )
}

/// The fixture workspace: default `tix.yaml` and five tickets covering valid,
/// invalid, and unknown-status cases.
pub fn fixture() -> MemHost {
    let mut h = MemHost::new();
    h.put(&format!("{WS}/tix.yaml"), DEFAULT_TIX_YAML);
    let tickets = [
        ticket(
            T_BACKLOG,
            "Nova product comparison article",
            "backlog",
            "2026-09-10T09:00:00Z",
            "client: nova\ntype: article\nowner: charlie\ndue: 2026-09-30\n",
            "Brief goes here.\n",
        ),
        ticket(
            T_PROGRESS,
            "Landing page refresh",
            "in_progress",
            "2026-09-11T09:00:00Z",
            "client: acme\ntype: landing_page\n",
            "",
        )
        .replace(
            "deliverables: []\n",
            "deliverables:\n- label: draft\n  ref: https://docs.google.com/document/d/abc\n- label: outline\n  ref: ./outline.md\n",
        ),
        ticket(T_DONE, "LinkedIn post", "done", "2026-09-09T09:00:00Z", "client: nova\ntype: linkedin\n", "Posted.\n"),
        ticket(T_NO_CLIENT, "Missing client", "backlog", "2026-09-12T09:00:00Z", "type: other\n", ""),
        ticket(
            T_UNKNOWN_STATUS,
            "Needs review",
            "review",
            "2026-09-08T09:00:00Z",
            "client: acme\ntype: article\n",
            "",
        ),
    ];
    let ids = [T_BACKLOG, T_PROGRESS, T_DONE, T_NO_CLIENT, T_UNKNOWN_STATUS];
    for (id, text) in ids.iter().zip(tickets.iter()) {
        h.put(&format!("{WS}/tickets/{id}/ticket.md"), text);
    }
    h
}

/// Runs `tix <args>` in `cwd` and compares a transcript against
/// `tests/golden/<name>.txt`. Set `TIX_UPDATE_GOLDEN=1` to (re)write goldens.
pub fn golden(name: &str, h: &mut MemHost, cwd: &str, args: &[&str]) -> Outcome {
    let o = h.run(cwd, args);
    let transcript = format!(
        "$ tix {}\nexit: {}\n--- stdout\n{}--- stderr\n{}",
        args.join(" "),
        o.code,
        o.stdout,
        o.stderr
    );
    let path: PathBuf = [
        env!("CARGO_MANIFEST_DIR"),
        "tests",
        "golden",
        &format!("{name}.txt"),
    ]
    .iter()
    .collect();
    if std::env::var("TIX_UPDATE_GOLDEN").is_ok() {
        std::fs::write(&path, &transcript).expect("write golden");
    } else {
        let expected = std::fs::read_to_string(&path).unwrap_or_else(|_| {
            panic!(
                "missing golden {}; run with TIX_UPDATE_GOLDEN=1",
                path.display()
            )
        });
        assert_eq!(transcript, expected, "golden mismatch: {name}");
    }
    o
}
