//! Shared comfy-table helpers for `ls` (TIX-16) and `board` (TIX-21).

use comfy_table::{presets, ContentArrangement, Table};

/// A borderless table (`NOTHING` preset) with disabled content arrangement,
/// used by `tix ls`.
pub fn plain_table() -> Table {
    let mut t = Table::new();
    t.load_style(presets::NOTHING);
    t.set_content_arrangement(ContentArrangement::Disabled);
    t
}

/// An ASCII-bordered, condensed table used by `tix board`.
pub fn board_table() -> Table {
    let mut t = Table::new();
    t.load_style(presets::ASCII_BORDERS_ONLY_CONDENSED);
    t.set_content_arrangement(ContentArrangement::Disabled);
    t
}

/// Renders a table as text: trims trailing whitespace on every line and ends
/// with exactly one trailing newline.
pub fn render(t: &Table) -> String {
    let mut out = String::new();
    for line in t.to_string().lines() {
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}
