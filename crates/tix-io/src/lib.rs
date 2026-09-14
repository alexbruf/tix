//! Unverified I/O layer: argv, YAML, frontmatter, tables, ULIDs, Storage (TIX-3).

pub mod schema_yaml;
pub mod storage;
pub mod ticket_md;
pub mod ulid;
pub mod workspace;

pub fn run(_argv: &[String], _cwd: &str) -> u32 {
    0
}
