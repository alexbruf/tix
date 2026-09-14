//! `tix attach` (TIX-20, TIX-13).

use crate::app::{emit_ticket, load_ticket, load_valid_schema, resolve_folder, save_ticket};
use crate::host::{CmdResult, Ctx, Failure, Host};
use crate::messages::ticket_error;
use clap::ArgMatches;
use tix_core::ops::attach;
use tix_core::Deliverable;

/// Last non-empty `/`-separated segment of `reference`, or `reference`
/// itself when there is none (TIX-13).
fn default_label(reference: &str) -> String {
    reference
        .split('/')
        .rfind(|s| !s.is_empty())
        .unwrap_or(reference)
        .to_string()
}

pub fn run<H: Host>(ctx: &mut Ctx<H>, m: &ArgMatches) -> CmdResult {
    let schema = load_valid_schema(ctx)?;
    let prefix = m.get_one::<String>("id").expect("required");
    let folder = resolve_folder(ctx, prefix)?;
    let t = load_ticket(ctx, &schema, &folder)?;

    let reference = m.get_one::<String>("ref").expect("required").clone();
    let label = match m.get_one::<String>("label") {
        Some(l) => l.clone(),
        None => default_label(&reference),
    };

    let now = ctx.host.now_unix();
    let d = Deliverable { label, reference };
    let mut candidate = t.clone();
    candidate.deliverables.push(d.clone());
    match attach(t, &schema, d, now) {
        Ok(t2) => {
            save_ticket(ctx, &schema, &t2)?;
            emit_ticket(ctx, &schema, &t2);
            Ok(())
        }
        Err(e) => Err(Failure::validation(ticket_error(&candidate, &schema, &e))),
    }
}
