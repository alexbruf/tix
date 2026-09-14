mod common;
use common::*;

#[test]
fn help_long_documents_agent_contract() {
    let o = fixture().run(WS, &["--help"]);
    assert_eq!(o.code, 0);
    for section in [
        "AGENT QUICKSTART",
        "--no-prompt",
        "JSON TICKET OBJECT",
        "EXIT CODES",
        "FILES",
    ] {
        assert!(o.stdout.contains(section), "missing {section}");
    }
}

#[test]
fn tix_15_new_help_lists_schema_fields() {
    let o = fixture().run(WS, &["new", "--help"]);
    assert_eq!(o.code, 0);
    assert!(o
        .stdout
        .contains("--type <article|landing_page|linkedin|other>"));
    assert!(o.stdout.contains("Required (string) [prompted if omitted]"));
    assert!(o.stdout.contains("--due <YYYY-MM-DD>"));
}

#[test]
fn every_command_has_examples_or_output_section() {
    let mut h = fixture();
    for cmd in [
        "init", "check", "new", "ls", "show", "mv", "set", "attach", "detach", "board", "path",
    ] {
        let o = h.run(WS, &[cmd, "--help"]);
        assert_eq!(o.code, 0, "{cmd}");
        assert!(o.stdout.contains("OUTPUT"), "{cmd} --help lacks OUTPUT");
    }
}
