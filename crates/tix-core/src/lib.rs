//! Verified core of tix. Pure functions over plain data; no I/O.
//! Every public function carries a Verus spec (TIX-2, TIX-24..TIX-28).

use vstd::prelude::*;

verus! {

pub fn placeholder() -> (r: u32)
    ensures r == 0,
{
    0
}

} // verus!
