//! Verified core of tix. Pure functions over plain data; no I/O.
//! Every public function carries a Verus spec (TIX-2, TIX-24..TIX-28).

pub mod ops;
pub mod query;
pub mod schema;
pub mod text;
pub mod ticket;
pub mod types;

pub use types::*;
