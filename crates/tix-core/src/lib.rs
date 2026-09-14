//! Verified core of tix. Pure functions over plain data; no I/O.
//! Every public function carries a Verus spec (TIX-2, TIX-24..TIX-28).
// Code shape follows what Verus can prove (no derived `==`, explicit loops), so
// clippy style and complexity suggestions do not apply here.
#![allow(clippy::style, clippy::complexity)]

pub mod board;
pub mod ops;
pub mod query;
pub mod schema;
pub mod text;
pub mod ticket;
pub mod types;

pub use types::*;
