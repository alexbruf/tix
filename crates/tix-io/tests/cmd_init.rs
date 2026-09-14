mod common;
use common::*;
use tix_io::schema_yaml::DEFAULT_TIX_YAML;
use tix_io::storage::Storage;
use tix_io::testkit::MemHost;

#[test]
fn tix_14_init_creates_workspace_in_fresh_dir() {
    let mut h = MemHost::new();
    let o = golden("init_fresh", &mut h, "/fresh", &["init"]);
    assert_eq!(o.code, 0);
    assert_eq!(h.file("/fresh/tix.yaml").as_deref(), Some(DEFAULT_TIX_YAML));
    assert!(h.fs.exists("/fresh/tickets").unwrap());
    assert_eq!(
        h.writes(),
        vec!["/fresh/tix.yaml".to_string(), "/fresh/tickets".to_string()]
    );
}

#[test]
fn tix_14_init_refuses_existing_workspace() {
    let mut h = fixture();
    let o = golden("init_already_exists", &mut h, WS, &["init"]);
    assert_eq!(o.code, 2);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_14_init_refuses_from_subdir_of_existing_workspace() {
    let mut h = fixture();
    let o = golden(
        "init_already_exists_subdir",
        &mut h,
        "/ws/tickets",
        &["init"],
    );
    assert_eq!(o.code, 2);
    assert!(h.writes().is_empty());
}

#[test]
fn tix_14_init_json() {
    let mut h = MemHost::new();
    let o = golden("init_json", &mut h, "/fresh2", &["init", "--json"]);
    assert_eq!(o.code, 0);
    assert!(h.fs.exists("/fresh2/tickets").unwrap());
}
