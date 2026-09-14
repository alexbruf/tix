//! `tix path` (TIX-23): prints the workspace-relative ticket folder.

use crate::app::{resolve_folder, ticket_dir};
use crate::host::{CmdResult, Ctx, Host};
use clap::ArgMatches;

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let prefix = m.get_one::<String>("id").expect("required");
    let path = ticket_dir(&resolve_folder(ctx, prefix)?);
    let text = if ctx.json {
        serde_json::json!({ "path": path }).to_string()
    } else {
        path
    };
    ctx.out(&format!("{text}\n"));
    Ok(())
}
