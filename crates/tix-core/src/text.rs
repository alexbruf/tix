//! Character-level predicates used by TIX-7, TIX-9 and TIX-11, each with an
//! executable check proved equal to its spec.

use vstd::prelude::*;

verus! {

pub open spec fn is_digit(c: char) -> bool {
    '0' <= c && c <= '9'
}

pub open spec fn is_lower(c: char) -> bool {
    'a' <= c && c <= 'z'
}

/// Uppercase Crockford base32 alphabet: digits and A-Z without I, L, O, U.
pub open spec fn is_crockford(c: char) -> bool {
    is_digit(c) || ('A' <= c && c <= 'Z' && c != 'I' && c != 'L' && c != 'O' && c != 'U')
}

/// TIX-7: 26 uppercase Crockford chars; first char at most `7` so the value fits 128 bits.
pub open spec fn valid_ulid(s: Seq<char>) -> bool {
    &&& s.len() == 26
    &&& '0' <= s[0] && s[0] <= '7'
    &&& forall|i: int| 0 <= i < 26 ==> is_crockford(#[trigger] s[i])
}

pub open spec fn digit_val(c: char) -> int {
    (c as int) - ('0' as int)
}

pub open spec fn is_leap(y: int) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

pub open spec fn days_in_month(y: int, m: int) -> int {
    if m == 2 {
        if is_leap(y) { 29 } else { 28 }
    } else if m == 4 || m == 6 || m == 9 || m == 11 {
        30
    } else {
        31
    }
}

pub open spec fn date_year(s: Seq<char>) -> int {
    digit_val(s[0]) * 1000 + digit_val(s[1]) * 100 + digit_val(s[2]) * 10 + digit_val(s[3])
}

pub open spec fn date_month(s: Seq<char>) -> int {
    digit_val(s[5]) * 10 + digit_val(s[6])
}

pub open spec fn date_day(s: Seq<char>) -> int {
    digit_val(s[8]) * 10 + digit_val(s[9])
}

/// TIX-8 / TIX-11 rule 4: `YYYY-MM-DD` naming a real calendar day.
pub open spec fn valid_date(s: Seq<char>) -> bool {
    &&& s.len() == 10
    &&& is_digit(s[0]) && is_digit(s[1]) && is_digit(s[2]) && is_digit(s[3])
    &&& s[4] == '-'
    &&& is_digit(s[5]) && is_digit(s[6])
    &&& s[7] == '-'
    &&& is_digit(s[8]) && is_digit(s[9])
    &&& 1 <= date_month(s) <= 12
    &&& 1 <= date_day(s) <= days_in_month(date_year(s), date_month(s))
}

/// TIX-9 rule 4 lexical part: `[a-z][a-z0-9_]*`.
pub open spec fn valid_ident(s: Seq<char>) -> bool {
    &&& s.len() >= 1
    &&& is_lower(s[0])
    &&& forall|i: int| 0 <= i < s.len() ==> (is_lower(#[trigger] s[i]) || is_digit(s[i]) || s[i] == '_')
}

/// TIX-9 rule 4: built-in ticket keys a schema field may not reuse.
pub open spec fn is_reserved(s: Seq<char>) -> bool {
    s == "id"@ || s == "title"@ || s == "status"@ || s == "created"@ || s == "updated"@
        || s == "deliverables"@
}

pub open spec fn valid_field_name(s: Seq<char>) -> bool {
    valid_ident(s) && !is_reserved(s)
}

/// TIX-9 rule 2.
pub open spec fn valid_group(s: Seq<char>) -> bool {
    s == "backlog"@ || s == "in_progress"@ || s == "completed"@
}

/// `s` starts with `p` (TIX-28).
pub open spec fn has_prefix(s: Seq<char>, p: Seq<char>) -> bool {
    p.len() <= s.len() && s.subrange(0, p.len() as int) == p
}

fn char_val(c: char) -> (r: u32)
    ensures
        r as int == c as int,
{
    c as u32
}

pub fn check_ulid(s: &String) -> (r: bool)
    ensures
        r == valid_ulid(s@),
{
    let st = s.as_str();
    if st.unicode_len() != 26 {
        return false;
    }
    let c0 = st.get_char(0);
    if !('0' <= c0 && c0 <= '7') {
        return false;
    }
    let mut i: usize = 0;
    while i < 26
        invariant
            st@ == s@,
            s@.len() == 26,
            i <= 26,
            forall|j: int| 0 <= j < i ==> is_crockford(#[trigger] s@[j]),
        decreases 26 - i,
    {
        let c = st.get_char(i);
        let ok = ('0' <= c && c <= '9') || ('A' <= c && c <= 'Z' && c != 'I' && c != 'L' && c != 'O'
            && c != 'U');
        if !ok {
            return false;
        }
        i = i + 1;
    }
    true
}

pub fn check_date(s: &String) -> (r: bool)
    ensures
        r == valid_date(s@),
{
    let st = s.as_str();
    if st.unicode_len() != 10 {
        return false;
    }
    let c0 = st.get_char(0);
    let c1 = st.get_char(1);
    let c2 = st.get_char(2);
    let c3 = st.get_char(3);
    let c4 = st.get_char(4);
    let c5 = st.get_char(5);
    let c6 = st.get_char(6);
    let c7 = st.get_char(7);
    let c8 = st.get_char(8);
    let c9 = st.get_char(9);
    let digits = '0' <= c0 && c0 <= '9' && '0' <= c1 && c1 <= '9' && '0' <= c2 && c2 <= '9' && '0'
        <= c3 && c3 <= '9' && '0' <= c5 && c5 <= '9' && '0' <= c6 && c6 <= '9' && '0' <= c8 && c8
        <= '9' && '0' <= c9 && c9 <= '9';
    if !(digits && c4 == '-' && c7 == '-') {
        return false;
    }
    let z = char_val('0');
    let y = (char_val(c0) - z) * 1000 + (char_val(c1) - z) * 100 + (char_val(c2) - z) * 10 + (
    char_val(c3) - z);
    let m = (char_val(c5) - z) * 10 + (char_val(c6) - z);
    let d = (char_val(c8) - z) * 10 + (char_val(c9) - z);
    assert(y as int == date_year(s@));
    assert(m as int == date_month(s@));
    assert(d as int == date_day(s@));
    if m < 1 || m > 12 {
        return false;
    }
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let dim: u32 = if m == 2 {
        if leap { 29 } else { 28 }
    } else if m == 4 || m == 6 || m == 9 || m == 11 {
        30
    } else {
        31
    };
    assert(dim as int == days_in_month(y as int, m as int));
    1 <= d && d <= dim
}

pub fn check_field_name(s: &String) -> (r: bool)
    ensures
        r == valid_field_name(s@),
{
    let st = s.as_str();
    let n = st.unicode_len();
    if n == 0 {
        return false;
    }
    let c0 = st.get_char(0);
    if !('a' <= c0 && c0 <= 'z') {
        return false;
    }
    let mut i: usize = 0;
    while i < n
        invariant
            st@ == s@,
            n == s@.len(),
            i <= n,
            forall|j: int| 0 <= j < i ==> (is_lower(#[trigger] s@[j]) || is_digit(s@[j]) || s@[j]
                == '_'),
        decreases n - i,
    {
        let c = st.get_char(i);
        if !(('a' <= c && c <= 'z') || ('0' <= c && c <= '9') || c == '_') {
            return false;
        }
        i = i + 1;
    }
    !is_reserved_exec(s)
}

fn is_reserved_exec(s: &String) -> (r: bool)
    ensures
        r == is_reserved(s@),
{
    str_is(s, "id") || str_is(s, "title") || str_is(s, "status") || str_is(s, "created") || str_is(
        s,
        "updated",
    ) || str_is(s, "deliverables")
}

pub fn check_group(s: &String) -> (r: bool)
    ensures
        r == valid_group(s@),
{
    str_is(s, "backlog") || str_is(s, "in_progress") || str_is(s, "completed")
}

/// String equality against a literal.
pub fn str_is(s: &String, lit: &str) -> (r: bool)
    ensures
        r == (s@ == lit@),
{
    let t = lit.to_owned();
    *s == t
}

pub fn str_eq(a: &String, b: &String) -> (r: bool)
    ensures
        r == (a@ == b@),
{
    *a == *b
}

pub fn starts_with(s: &String, p: &String) -> (r: bool)
    ensures
        r == has_prefix(s@, p@),
{
    let ss = s.as_str();
    let ps = p.as_str();
    let n = ss.unicode_len();
    let m = ps.unicode_len();
    if m > n {
        return false;
    }
    let mut i: usize = 0;
    while i < m
        invariant
            ss@ == s@,
            ps@ == p@,
            n == s@.len(),
            m == p@.len(),
            m <= n,
            i <= m,
            forall|j: int| 0 <= j < i ==> s@[j] == p@[j],
        decreases m - i,
    {
        if ss.get_char(i) != ps.get_char(i) {
            assert(s@.subrange(0, m as int)[i as int] != p@[i as int]);
            return false;
        }
        i = i + 1;
    }
    assert(s@.subrange(0, m as int) =~= p@);
    true
}

} // verus!
