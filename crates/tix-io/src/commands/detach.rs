//! `tix detach` (TIX-20).

use crate::app::{emit_ticket, load_ticket, load_valid_schema, resolve_folder, save_ticket};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::messages::ticket_error;
use clap::ArgMatches;
use tix_core::ops::{detach, DetachError};

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let prefix = m.get_one::<String>("id").expect("required");
    let folder = resolve_folder(ctx, prefix)?;
    let t = load_ticket(ctx, &schema, &folder)?;
    let target = m.get_one::<String>("target").expect("required").clone();

    let now = ctx.host.now_unix();
    let original = t.clone();
    match detach(t, &schema, &target, now) {
        Ok(t2) => {
            save_ticket(ctx, &schema, &t2)?;
            emit_ticket(ctx, &schema, &t2);
            Ok(())
        }
        Err(DetachError::NoMatch) => Err(Failure::validation(format!(
            "no deliverable matches '{target}'"
        ))),
        Err(DetachError::Ambiguous) => Err(Failure::validation(format!(
            "'{target}' matches more than one deliverable"
        ))),
        Err(DetachError::Invalid(e)) => {
            let mut candidate = original;
            candidate
                .deliverables
                .retain(|d| d.reference != target && d.label != target);
            Err(Failure::validation(ticket_error(&candidate, &schema, &e)))
        }
    }
}
