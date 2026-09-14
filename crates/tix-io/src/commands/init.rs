//! `tix init` (TIX-14).

use crate::app::{SCHEMA_FILE, TICKETS_DIR};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::schema_yaml::DEFAULT_TIX_YAML;
use crate::storage::Storage;
use crate::workspace::find_workspace;
use clap::ArgMatches;

pub fn run<H: Host>(ctx: &mut Ctx<H>, _m: &ArgMatches) -> CmdResult {
    if let Some(dir) = find_workspace(&*ctx.host, &ctx.root)? {
        return Err(Failure::usage(format!(
            "{SCHEMA_FILE} already exists at {dir}"
        )));
    }

    ctx.write(SCHEMA_FILE, DEFAULT_TIX_YAML.as_bytes())?;
    ctx.mkdir_all(TICKETS_DIR)?;

    if ctx.json {
        let text = serde_json::json!({ "root": ctx.root }).to_string();
        ctx.out(&format!("{text}\n"));
    } else {
        let text = format!("initialized {}\n", ctx.root);
        ctx.out(&text);
    }
    Ok(())
}
