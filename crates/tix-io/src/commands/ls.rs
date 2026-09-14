//! `tix ls`.

use crate::host::{CmdResult, Ctx, Failure, Host};
use clap::ArgMatches;

pub fn run<H: Host>(_ctx: &mut Ctx<H>, _m: &ArgMatches) -> CmdResult {
    Err(Failure::usage("tix ls is not implemented yet"))
}
