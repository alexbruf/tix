//! `tix mv` (TIX-18).

use crate::app::{emit_ticket, load_ticket, load_valid_schema, resolve_folder, save_ticket};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::messages::ticket_error;
use clap::ArgMatches;
use tix_core::ops::transition;

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let prefix = m.get_one::<String>("id").expect("required");
    let folder = resolve_folder(ctx, prefix)?;
    let t = load_ticket(ctx, &schema, &folder)?;
    let status = m.get_one::<String>("status").expect("required").clone();

    let now = ctx.host.now_unix();
    let mut candidate = t.clone();
    candidate.status = status.clone();
    match transition(t, &schema, status, now) {
        Ok(t2) => {
            save_ticket(ctx, &schema, &t2)?;
            emit_ticket(ctx, &schema, &t2);
            Ok(())
        }
        Err(e) => Err(Failure::validation(ticket_error(&candidate, &schema, &e))),
    }
}
