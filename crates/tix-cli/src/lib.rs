//! `StdHost`: the `tix-io` host interface implemented over `std`. Shared by
//! the native `tix` binary and `tix-tui` so the real-filesystem `Host` impl
//! exists in one place.

pub mod host;
