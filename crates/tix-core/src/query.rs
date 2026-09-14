//! TIX-28 id prefix resolution and TIX-27 filtering.

use crate::text::*;
use crate::types::*;
use vstd::prelude::*;

verus! {

// ------------------------------------------------------------------- resolve

#[derive(Debug, PartialEq, Eq)]
pub enum ResolveError {
    TooShort,
    NotFound,
    /// Indices into `ids` of every id carrying the prefix, ascending.
    Ambiguous(Vec<usize>),
}

pub open spec fn increasing(v: Seq<usize>) -> bool {
    forall|a: int, b: int| 0 <= a < b < v.len() ==> (#[trigger] v[a]) < (#[trigger] v[b])
}

/// `tix` id prefix lookup (TIX-7, TIX-28). Returns an index into `ids`.
pub fn resolve(prefix: &String, ids: &Vec<String>) -> (r: Result<usize, ResolveError>)
    ensures
        // TIX-28: TooShort iff prefix shorter than 4.
        (r matches Err(ResolveError::TooShort)) <==> prefix@.len() < 4,
        // TIX-28: Ok(id) only when exactly one id starts with prefix.
        r matches Ok(i) ==> prefix@.len() >= 4 && 0 <= i < ids@.len() && has_prefix(ids@[i as int]@, prefix@)
            && forall|j: int| 0 <= j < ids@.len() && j != i ==> !has_prefix((#[trigger] ids@[j])@, prefix@),
        // TIX-28: NotFound iff none does.
        r matches Err(ResolveError::NotFound) ==> prefix@.len() >= 4 && forall|j: int|
            0 <= j < ids@.len() ==> !has_prefix((#[trigger] ids@[j])@, prefix@),
        // TIX-28: Ambiguous carries exactly the matching set, which has at least two members.
        r matches Err(ResolveError::Ambiguous(c)) ==> prefix@.len() >= 4 && c@.len() >= 2 && increasing(c@)
            && (forall|a: int| 0 <= a < c@.len() ==> (#[trigger] c@[a]) < ids@.len()) && forall|j: int|
            0 <= j < ids@.len() ==> (has_prefix((#[trigger] ids@[j])@, prefix@) <==> c@.contains(j as usize)),
{
    if prefix.as_str().unicode_len() < 4 {
        return Err(ResolveError::TooShort);
    }
    let n = ids.len();
    let mut c: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            n == ids@.len(),
            i <= n,
            increasing(c@),
            forall|a: int| 0 <= a < c@.len() ==> (#[trigger] c@[a]) < i,
            forall|j: int| 0 <= j < i ==> (has_prefix((#[trigger] ids@[j])@, prefix@) <==> c@.contains(j as usize)),
        decreases n - i,
    {
        let ghost c0 = c@;
        if starts_with(&ids[i], prefix) {
            c.push(i);
            assert(c@[c0.len() as int] == i);
            assert forall|j: int| 0 <= j < i + 1 implies (has_prefix((#[trigger] ids@[j])@, prefix@)
                <==> c@.contains(j as usize)) by {
                if j < i {
                    if c0.contains(j as usize) {
                        let a = choose|a: int| 0 <= a < c0.len() && c0[a] == j as usize;
                        assert(c@[a] == j as usize);
                    }
                    if c@.contains(j as usize) {
                        let a = choose|a: int| 0 <= a < c@.len() && c@[a] == j as usize;
                        assert(a < c0.len());
                        assert(c0[a] == j as usize);
                    }
                }
            }
        } else {
            assert forall|j: int| 0 <= j < i + 1 implies (has_prefix((#[trigger] ids@[j])@, prefix@)
                <==> c@.contains(j as usize)) by {
                if j == i && c@.contains(j as usize) {
                    let a = choose|a: int| 0 <= a < c@.len() && c@[a] == j as usize;
                }
            }
        }
        i = i + 1;
    }
    if c.len() == 0 {
        assert forall|j: int| 0 <= j < ids@.len() implies !has_prefix((#[trigger] ids@[j])@, prefix@) by {
            assert(!c@.contains(j as usize));
        }
        return Err(ResolveError::NotFound);
    }
    if c.len() == 1 {
        let only = c[0];
        assert forall|j: int| 0 <= j < ids@.len() && j != only implies !has_prefix(
            (#[trigger] ids@[j])@,
            prefix@,
        ) by {
            if c@.contains(j as usize) {
                let a = choose|a: int| 0 <= a < c@.len() && c@[a] == j as usize;
            }
        }
        assert(c@.contains(only));
        return Ok(only);
    }
    Err(ResolveError::Ambiguous(c))
}

// -------------------------------------------------------------------- filter

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TokenKey {
    Status,
    Group,
    /// Index into `Schema::fields`.
    Field(usize),
}

/// One parsed `KEY:VALUE` filter token.
#[derive(Debug, PartialEq, Eq)]
pub struct Token {
    pub key: TokenKey,
    pub value: String,
}

pub open spec fn first_field_named(s: Schema, name: Seq<char>, k: int) -> bool {
    &&& 0 <= k < s.fields@.len()
    &&& s.fields@[k].name@ == name
    &&& forall|j: int| 0 <= j < k ==> (#[trigger] s.fields@[j]).name@ != name
}

pub open spec fn field_declared(s: Schema, name: Seq<char>) -> bool {
    exists|k: int| 0 <= k < s.fields@.len() && (#[trigger] s.fields@[k]).name@ == name
}

/// TIX-27: parses a token key; `None` (a parse error) iff the key is unknown.
pub fn parse_token(s: &Schema, key: &String, value: String) -> (r: Option<Token>)
    ensures
        r is None <==> !(key@ == "status"@ || key@ == "group"@ || field_declared(*s, key@)),  // TIX-27
        r matches Some(tok) ==> tok.value == value && match tok.key {
            TokenKey::Status => key@ == "status"@,
            TokenKey::Group => key@ == "group"@,
            TokenKey::Field(k) => key@ != "status"@ && key@ != "group"@ && first_field_named(
                *s,
                key@,
                k as int,
            ),
        },
{
    if str_is(key, "status") {
        return Some(Token { key: TokenKey::Status, value });
    }
    if str_is(key, "group") {
        return Some(Token { key: TokenKey::Group, value });
    }
    let fs = &s.fields;
    let mut k: usize = 0;
    while k < fs.len()
        invariant
            fs == &s.fields,
            key@ != "status"@,
            key@ != "group"@,
            k <= fs@.len(),
            forall|j: int| 0 <= j < k ==> (#[trigger] fs@[j]).name@ != key@,
        decreases fs@.len() - k,
    {
        if str_eq(&fs[k].name, key) {
            return Some(Token { key: TokenKey::Field(k), value });
        }
        k = k + 1;
    }
    None
}

pub open spec fn entry_matches(ty: FieldType, v: Value, x: Seq<char>) -> bool {
    match v {
        Value::Str(y) => ty != FieldType::List && y@ == x,
        Value::List(ys) => ty == FieldType::List && exists|i: int| 0 <= i < ys@.len() && (#[trigger] ys@[i])@ == x,
    }
}

/// TIX-27 `matches` for a single token.
pub open spec fn token_matches(t: Ticket, s: Schema, tok: Token) -> bool {
    match tok.key {
        TokenKey::Status => t.status@ == tok.value@,
        TokenKey::Group => exists|i: int|
            0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == t.status@ && s.statuses@[i].group@
                == tok.value@,
        TokenKey::Field(k) => 0 <= k < s.fields@.len() && exists|j: int|
            0 <= j < t.fields@.len() && (#[trigger] t.fields@[j]).name@ == s.fields@[k as int].name@
                && entry_matches(s.fields@[k as int].ty, t.fields@[j].value, tok.value@),
    }
}

/// TIX-27 `matches(t, q)`: conjunction over tokens.
pub open spec fn matches(t: Ticket, s: Schema, q: Seq<Token>) -> bool {
    forall|k: int| 0 <= k < q.len() ==> token_matches(t, s, #[trigger] q[k])
}

fn check_entry(ty: FieldType, v: &Value, x: &String) -> (r: bool)
    ensures
        r == entry_matches(ty, *v, x@),
{
    match v {
        Value::Str(y) => match ty {
            FieldType::List => false,
            _ => str_eq(y, x),
        },
        Value::List(ys) => match ty {
            FieldType::List => crate::schema::vec_contains_str(ys, x),
            _ => false,
        },
    }
}

fn check_token(t: &Ticket, s: &Schema, tok: &Token) -> (r: bool)
    ensures
        r == token_matches(*t, *s, *tok),
{
    match tok.key {
        TokenKey::Status => str_eq(&t.status, &tok.value),
        TokenKey::Group => {
            let st = &s.statuses;
            let mut i: usize = 0;
            while i < st.len()
                invariant
                    st == &s.statuses,
                    tok.key == TokenKey::Group,
                    i <= st@.len(),
                    forall|j: int|
                        0 <= j < i ==> !((#[trigger] st@[j]).name@ == t.status@ && st@[j].group@ == tok.value@),
                decreases st@.len() - i,
            {
                if str_eq(&st[i].name, &t.status) && str_eq(&st[i].group, &tok.value) {
                    assert((st@[i as int]).name@ == t.status@);
                    return true;
                }
                i = i + 1;
            }
            false
        },
        TokenKey::Field(k) => {
            if k >= s.fields.len() {
                return false;
            }
            let f = &s.fields[k];
            let tf = &t.fields;
            let mut j: usize = 0;
            while j < tf.len()
                invariant
                    tf == &t.fields,
                    tok.key == TokenKey::Field(k),
                    f == &s.fields@[k as int],
                    k < s.fields@.len(),
                    j <= tf@.len(),
                    forall|x: int|
                        0 <= x < j ==> !((#[trigger] tf@[x]).name@ == f.name@ && entry_matches(
                            f.ty,
                            tf@[x].value,
                            tok.value@,
                        )),
                decreases tf@.len() - j,
            {
                if str_eq(&tf[j].name, &f.name) && check_entry(f.ty, &tf[j].value, &tok.value) {
                    assert((tf@[j as int]).name@ == s.fields@[k as int].name@);
                    return true;
                }
                j = j + 1;
            }
            false
        },
    }
}

fn check_matches(t: &Ticket, s: &Schema, q: &Vec<Token>) -> (r: bool)
    ensures
        r == matches(*t, *s, q@),
{
    let mut k: usize = 0;
    while k < q.len()
        invariant
            k <= q@.len(),
            forall|x: int| 0 <= x < k ==> token_matches(*t, *s, #[trigger] q@[x]),
        decreases q@.len() - k,
    {
        if !check_token(t, s, &q[k]) {
            assert(!token_matches(*t, *s, q@[k as int]));
            return false;
        }
        k = k + 1;
    }
    true
}

/// TIX-27: indices of exactly the tickets matching `q`, in input order.
pub fn filter(tickets: &Vec<Ticket>, s: &Schema, q: &Vec<Token>) -> (r: Vec<usize>)
    ensures
        increasing(r@),  // TIX-27: input order
        forall|a: int| 0 <= a < r@.len() ==> (#[trigger] r@[a]) < tickets@.len(),
        forall|i: int|
            0 <= i < tickets@.len() ==> (matches(#[trigger] tickets@[i], *s, q@) <==> r@.contains(i as usize)),  // TIX-27
{
    let n = tickets.len();
    let mut r: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            n == tickets@.len(),
            i <= n,
            increasing(r@),
            forall|a: int| 0 <= a < r@.len() ==> (#[trigger] r@[a]) < i,
            forall|j: int| 0 <= j < i ==> (matches(#[trigger] tickets@[j], *s, q@) <==> r@.contains(j as usize)),
        decreases n - i,
    {
        let ghost r0 = r@;
        if check_matches(&tickets[i], s, q) {
            r.push(i);
            assert(r@[r0.len() as int] == i);
            assert forall|j: int| 0 <= j < i + 1 implies (matches(#[trigger] tickets@[j], *s, q@)
                <==> r@.contains(j as usize)) by {
                if j < i {
                    if r0.contains(j as usize) {
                        let a = choose|a: int| 0 <= a < r0.len() && r0[a] == j as usize;
                        assert(r@[a] == j as usize);
                    }
                    if r@.contains(j as usize) {
                        let a = choose|a: int| 0 <= a < r@.len() && r@[a] == j as usize;
                        assert(a < r0.len());
                        assert(r0[a] == j as usize);
                    }
                }
            }
        } else {
            assert forall|j: int| 0 <= j < i + 1 implies (matches(#[trigger] tickets@[j], *s, q@)
                <==> r@.contains(j as usize)) by {
                if j == i && r@.contains(j as usize) {
                    let a = choose|a: int| 0 <= a < r@.len() && r@[a] == j as usize;
                }
            }
        }
        i = i + 1;
    }
    r
}

} // verus!
