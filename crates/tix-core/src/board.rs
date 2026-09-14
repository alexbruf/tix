//! Ordering and board layout: TIX-16 row order, TIX-21 / TIX-26 board partition.
//! All results are indices into the caller's `Vec<Ticket>`.

#[allow(unused_imports)] // spec-only items
use crate::schema::*;
use crate::text::*;
#[allow(unused_imports)] // spec-only items
use crate::ticket::*;
use crate::types::*;
use vstd::prelude::*;

verus! {

// ------------------------------------------------------------ string order

/// Lexicographic order on character sequences (used for `id` tie-breaks).
pub open spec fn lex_lt(a: Seq<char>, b: Seq<char>) -> bool
    decreases a.len(),
{
    if b.len() == 0 {
        false
    } else if a.len() == 0 {
        true
    } else if a[0] != b[0] {
        a[0] < b[0]
    } else {
        lex_lt(a.drop_first(), b.drop_first())
    }
}

pub proof fn lemma_lex_irrefl(a: Seq<char>)
    ensures
        !lex_lt(a, a),
    decreases a.len(),
{
    if a.len() > 0 {
        lemma_lex_irrefl(a.drop_first());
    }
}

pub proof fn lemma_lex_trans(a: Seq<char>, b: Seq<char>, c: Seq<char>)
    requires
        lex_lt(a, b),
        lex_lt(b, c),
    ensures
        lex_lt(a, c),
    decreases a.len(),
{
    if a.len() > 0 && a[0] == b[0] && b[0] == c[0] {
        lemma_lex_trans(a.drop_first(), b.drop_first(), c.drop_first());
    }
}

pub fn lex_less(a: &String, b: &String) -> (r: bool)
    ensures
        r == lex_lt(a@, b@),
{
    let sa = a.as_str();
    let sb = b.as_str();
    let na = sa.unicode_len();
    let nb = sb.unicode_len();
    let mut i: usize = 0;
    assert(a@.subrange(0, na as int) =~= a@);
    assert(b@.subrange(0, nb as int) =~= b@);
    loop
        invariant
            sa@ == a@,
            sb@ == b@,
            na == a@.len(),
            nb == b@.len(),
            i <= na,
            i <= nb,
            lex_lt(a@, b@) == lex_lt(a@.subrange(i as int, na as int), b@.subrange(i as int, nb as int)),
        decreases na - i,
    {
        let ghost ta = a@.subrange(i as int, na as int);
        let ghost tb = b@.subrange(i as int, nb as int);
        if i == nb {
            return false;
        }
        if i == na {
            return true;
        }
        let ca = sa.get_char(i);
        let cb = sb.get_char(i);
        assert(ta[0] == ca && tb[0] == cb);
        if ca != cb {
            return ca < cb;
        }
        assert(ta.drop_first() =~= a@.subrange(i + 1, na as int));
        assert(tb.drop_first() =~= b@.subrange(i + 1, nb as int));
        i = i + 1;
    }
}

// ----------------------------------------------------------------- sorting

/// `created` ascending, ties by `id` (TIX-16, TIX-21, TIX-26).
pub open spec fn ct_lt(x: Ticket, y: Ticket) -> bool {
    x.created < y.created || (x.created == y.created && lex_lt(x.id@, y.id@))
}

/// `rank` first, then `ct_lt`.
pub open spec fn key_lt(ts: Seq<Ticket>, rank: Seq<usize>, x: usize, y: usize) -> bool {
    rank[x as int] < rank[y as int] || (rank[x as int] == rank[y as int] && ct_lt(ts[x as int], ts[y as int]))
}

pub open spec fn distinct(v: Seq<usize>) -> bool {
    forall|p: int, q: int| 0 <= p < q < v.len() ==> (#[trigger] v[p]) != (#[trigger] v[q])
}

pub open spec fn in_range(v: Seq<usize>, n: int) -> bool {
    forall|p: int| 0 <= p < v.len() ==> (#[trigger] v[p]) < n
}

pub open spec fn sorted_by(ts: Seq<Ticket>, rank: Seq<usize>, v: Seq<usize>) -> bool {
    forall|p: int, q: int| 0 <= p < q < v.len() ==> !key_lt(ts, rank, #[trigger] v[q], #[trigger] v[p])
}

pub open spec fn ct_sorted(ts: Seq<Ticket>, v: Seq<usize>) -> bool {
    forall|p: int, q: int| 0 <= p < q < v.len() ==> !ct_lt(ts[(#[trigger] v[q]) as int], ts[(#[trigger] v[p]) as int])
}

proof fn lemma_key_irrefl(ts: Seq<Ticket>, rank: Seq<usize>, x: usize)
    ensures
        !key_lt(ts, rank, x, x),
{
    lemma_lex_irrefl(ts[x as int].id@);
}

proof fn lemma_key_trans(ts: Seq<Ticket>, rank: Seq<usize>, x: usize, y: usize, z: usize)
    requires
        key_lt(ts, rank, x, y),
        key_lt(ts, rank, y, z),
    ensures
        key_lt(ts, rank, x, z),
{
    let (tx, ty, tz) = (ts[x as int], ts[y as int], ts[z as int]);
    if rank[x as int] == rank[y as int] && rank[y as int] == rank[z as int] && tx.created == ty.created
        && ty.created == tz.created {
        lemma_lex_trans(tx.id@, ty.id@, tz.id@);
    }
}

fn key_less(ts: &Vec<Ticket>, rank: &Vec<usize>, x: usize, y: usize) -> (r: bool)
    requires
        x < ts@.len(),
        y < ts@.len(),
        rank@.len() == ts@.len(),
    ensures
        r == key_lt(ts@, rank@, x, y),
{
    let rx = rank[x];
    let ry = rank[y];
    if rx != ry {
        return rx < ry;
    }
    let tx = &ts[x];
    let ty = &ts[y];
    if tx.created != ty.created {
        return tx.created < ty.created;
    }
    lex_less(&tx.id, &ty.id)
}

/// Sorts `idx` by `key_lt` (a permutation of `idx`).
pub fn sort_indices(ts: &Vec<Ticket>, rank: &Vec<usize>, idx: &Vec<usize>) -> (r: Vec<usize>)
    requires
        rank@.len() == ts@.len(),
        in_range(idx@, ts@.len() as int),
        distinct(idx@),
    ensures
        r@.len() == idx@.len(),
        in_range(r@, ts@.len() as int),
        distinct(r@),
        sorted_by(ts@, rank@, r@),
        forall|v: usize| r@.contains(v) <==> idx@.contains(v),
{
    let n = idx.len();
    let mut r: Vec<usize> = Vec::new();
    let mut k: usize = 0;
    while k < n
        invariant
            n == idx@.len(),
            rank@.len() == ts@.len(),
            in_range(idx@, ts@.len() as int),
            distinct(idx@),
            k <= n,
            r@.len() == k,
            in_range(r@, ts@.len() as int),
            distinct(r@),
            sorted_by(ts@, rank@, r@),
            forall|v: usize| r@.contains(v) <==> idx@.subrange(0, k as int).contains(v),
        decreases n - k,
    {
        let x = idx[k];
        let mut p: usize = 0;
        loop
            invariant
                rank@.len() == ts@.len(),
                in_range(r@, ts@.len() as int),
                x < ts@.len(),
                p <= r@.len(),
                forall|q: int| 0 <= q < p ==> !key_lt(ts@, rank@, x, #[trigger] r@[q]),
            ensures
                p <= r@.len(),
                forall|q: int| 0 <= q < p ==> !key_lt(ts@, rank@, x, #[trigger] r@[q]),
                p < r@.len() ==> key_lt(ts@, rank@, x, r@[p as int]),
            decreases r@.len() - p,
        {
            if p == r.len() {
                break;
            }
            if key_less(ts, rank, x, r[p]) {
                break;
            }
            p = p + 1;
        }
        let ghost old_r = r@;
        proof {
            // x is new: r holds only idx[0..k].
            if old_r.contains(x) {
                let a = choose|a: int| 0 <= a < k && idx@.subrange(0, k as int)[a] == x;
                assert(idx@[a] == idx@[k as int]);
            }
        }
        r.insert(p, x);
        proof {
            old_r.insert_ensures(p as int, x);
            let nr = r@;
            assert(nr == old_r.insert(p as int, x));
            assert forall|q: int| 0 <= q < nr.len() implies (#[trigger] nr[q]) < ts@.len() by {
                if q > p {
                    assert(nr[q] == old_r[q - 1]);
                }
            }
            assert forall|p1: int, q1: int| 0 <= p1 < q1 < nr.len() implies (#[trigger] nr[p1]) != (#[trigger] nr[q1]) by {
                if q1 == p {
                    assert(old_r.contains(nr[p1]));
                } else if p1 == p {
                    assert(nr[q1] == old_r[q1 - 1]);
                    assert(old_r.contains(nr[q1]));
                } else if p1 > p {
                    assert(nr[p1] == old_r[p1 - 1] && nr[q1] == old_r[q1 - 1]);
                } else if q1 > p {
                    assert(nr[q1] == old_r[q1 - 1]);
                }
            }
            assert forall|p1: int, q1: int| 0 <= p1 < q1 < nr.len() implies !key_lt(
                ts@,
                rank@,
                #[trigger] nr[q1],
                #[trigger] nr[p1],
            ) by {
                if q1 < p {
                } else if q1 == p {
                } else if p1 > p {
                    assert(nr[p1] == old_r[p1 - 1] && nr[q1] == old_r[q1 - 1]);
                } else if p1 < p {
                    assert(nr[q1] == old_r[q1 - 1]);
                } else {
                    // p1 == p, nr[p1] == x; the scan stopped because x < old_r[p].
                    let y = old_r[q1 - 1];
                    assert(nr[q1] == y);
                    assert(key_lt(ts@, rank@, x, old_r[p as int]));
                    if key_lt(ts@, rank@, y, x) {
                        lemma_key_trans(ts@, rank@, y, x, old_r[p as int]);
                        if q1 - 1 == p {
                            lemma_key_irrefl(ts@, rank@, y);
                        }
                    }
                }
            }
            let pre = idx@.subrange(0, k as int + 1);
            assert forall|v: usize| nr.contains(v) <==> pre.contains(v) by {
                if nr.contains(v) {
                    let q = choose|q: int| 0 <= q < nr.len() && nr[q] == v;
                    if q < p {
                        assert(old_r.contains(v));
                    } else if q > p {
                        assert(nr[q] == old_r[q - 1]);
                        assert(old_r.contains(v));
                    } else {
                        assert(pre[k as int] == v);
                    }
                    if old_r.contains(v) {
                        let a = choose|a: int| 0 <= a < k && idx@.subrange(0, k as int)[a] == v;
                        assert(pre[a] == v);
                    }
                }
                if pre.contains(v) {
                    let a = choose|a: int| 0 <= a <= k && pre[a] == v;
                    if a < k {
                        assert(idx@.subrange(0, k as int)[a] == v);
                        assert(old_r.contains(v));
                        let q = choose|q: int| 0 <= q < old_r.len() && old_r[q] == v;
                        if q < p {
                            assert(nr[q] == v);
                        } else {
                            assert(nr[q + 1] == v);
                        }
                    } else {
                        assert(nr[p as int] == v);
                    }
                }
            }
        }
        k = k + 1;
    }
    assert(idx@.subrange(0, n as int) =~= idx@);
    r
}

// ---------------------------------------------------------------- ranking

/// `r` is the index of the first status named `name`, or `statuses.len()` if none.
pub open spec fn status_rank_of(s: Schema, name: Seq<char>, r: int) -> bool {
    &&& 0 <= r <= s.statuses@.len()
    &&& forall|j: int| 0 <= j < r ==> (#[trigger] s.statuses@[j]).name@ != name
    &&& r < s.statuses@.len() ==> s.statuses@[r].name@ == name
}

/// TIX-16: per-ticket status rank; sorting by it gives schema status order, unknown last.
pub fn status_ranks(ts: &Vec<Ticket>, s: &Schema) -> (ranks: Vec<usize>)
    ensures
        ranks@.len() == ts@.len(),
        forall|i: int| 0 <= i < ts@.len() ==> status_rank_of(*s, (#[trigger] ts@[i]).status@, ranks@[i] as int),
{
    let st = &s.statuses;
    let mut ranks: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < ts.len()
        invariant
            st == &s.statuses,
            i <= ts@.len(),
            ranks@.len() == i,
            forall|x: int| 0 <= x < i ==> status_rank_of(*s, (#[trigger] ts@[x]).status@, ranks@[x] as int),
        decreases ts@.len() - i,
    {
        let name = &ts[i].status;
        let mut r: usize = 0;
        while r < st.len() && !str_eq(&st[r].name, name)
            invariant
                st == &s.statuses,
                r <= st@.len(),
                forall|j: int| 0 <= j < r ==> (#[trigger] st@[j]).name@ != name@,
            decreases st@.len() - r,
        {
            r = r + 1;
        }
        ranks.push(r);
        assert(ranks@[i as int] == r);
        i = i + 1;
    }
    ranks
}

proof fn lemma_rank_unique(s: Schema, name: Seq<char>, r: int, c: int)
    requires
        schema_rule1_names(s),
        status_rank_of(s, name, r),
        0 <= c < s.statuses@.len(),
    ensures
        (r == c) <==> s.statuses@[c].name@ == name,
{
    if s.statuses@[c].name@ == name && r != c {
        if r < c {
            assert(s.statuses@[r].name@ == s.statuses@[c].name@);
        }
    }
}

/// Status names are unique (the part of TIX-9 rule 1 the board needs).
pub open spec fn schema_rule1_names(s: Schema) -> bool {
    forall|a: int, b: int|
        0 <= a < b < s.statuses@.len() ==> (#[trigger] s.statuses@[a]).name@ != (#[trigger] s.statuses@[b]).name@
}

// ------------------------------------------------------------------ board

pub struct Board {
    /// One column per status (or per group with `--group`), in order.
    pub columns: Vec<Vec<usize>>,
    /// The synthetic `?` column: tickets whose status is not in the schema.
    pub unknown: Vec<usize>,
}

/// Entries of `idx` whose bucket is `c`, in `idx` order.
#[allow(unused_variables)] // `n` is spec-only
fn select(idx: &Vec<usize>, bucket: &Vec<usize>, c: usize, n: usize) -> (r: Vec<usize>)
    requires
        bucket@.len() == n,
        in_range(idx@, n as int),
        distinct(idx@),
    ensures
        in_range(r@, n as int),
        distinct(r@),
        forall|v: usize| r@.contains(v) <==> (idx@.contains(v) && bucket@[v as int] == c),
{
    let mut r: Vec<usize> = Vec::new();
    let mut k: usize = 0;
    while k < idx.len()
        invariant
            bucket@.len() == n,
            in_range(idx@, n as int),
            distinct(idx@),
            k <= idx@.len(),
            in_range(r@, n as int),
            distinct(r@),
            forall|v: usize| r@.contains(v) <==> (idx@.subrange(0, k as int).contains(v) && bucket@[v as int] == c),
        decreases idx@.len() - k,
    {
        let v = idx[k];
        let ghost r0 = r@;
        assert(forall|w: usize| r0.contains(w) <==> (idx@.subrange(0, k as int).contains(w) && bucket@[w as int] == c));
        let ghost pre = idx@.subrange(0, k as int);
        let ghost pre1 = idx@.subrange(0, k as int + 1);
        proof {
            assert forall|w: usize| pre1.contains(w) <==> (pre.contains(w) || w == v) by {
                if pre1.contains(w) {
                    let a = choose|a: int| 0 <= a <= k && pre1[a] == w;
                    if a < k {
                        assert(pre[a] == w);
                    }
                }
                if pre.contains(w) {
                    let a = choose|a: int| 0 <= a < k && pre[a] == w;
                    assert(pre1[a] == w);
                }
                if w == v {
                    assert(pre1[k as int] == w);
                }
            }
            if pre.contains(v) {
                let a = choose|a: int| 0 <= a < k && pre[a] == v;
                assert(idx@[a] == idx@[k as int]);
            }
        }
        if bucket[v] == c {
            r.push(v);
            proof {
                assert forall|w: usize| r@.contains(w) <==> (r0.contains(w) || w == v) by {
                    if r@.contains(w) {
                        let a = choose|a: int| 0 <= a < r@.len() && r@[a] == w;
                        if a < r0.len() {
                            assert(r0[a] == w);
                        }
                    }
                    if r0.contains(w) {
                        let a = choose|a: int| 0 <= a < r0.len() && r0[a] == w;
                        assert(r@[a] == w);
                    }
                    if w == v {
                        assert(r@[r0.len() as int] == w);
                    }
                }
                assert forall|p: int, q: int| 0 <= p < q < r@.len() implies (#[trigger] r@[p]) != (#[trigger] r@[q]) by {
                    if q == r0.len() {
                        assert(r0.contains(r@[p]));
                    } else {
                        assert(r@[p] == r0[p] && r@[q] == r0[q]);
                    }
                }
                assert forall|p: int| 0 <= p < r@.len() implies (#[trigger] r@[p]) < n by {
                    if p < r0.len() {
                        assert(r@[p] == r0[p]);
                    }
                }
            }
        }
        assert forall|w: usize| r@.contains(w) <==> (pre1.contains(w) && bucket@[w as int] == c) by {
            assert(r0.contains(w) <==> (pre.contains(w) && bucket@[w as int] == c));
            assert(pre1.contains(w) <==> (pre.contains(w) || w == v));
            if bucket@[v as int] == c {
                assert(r@.contains(w) <==> (r0.contains(w) || w == v));
            } else {
                assert(r@ == r0);
            }
        }
        assert(pre1 == idx@.subrange(0, (k + 1) as int));
        k = k + 1;
    }
    assert(idx@.subrange(0, idx@.len() as int) =~= idx@);
    r
}

proof fn lemma_sorted_same_rank(ts: Seq<Ticket>, rank: Seq<usize>, v: Seq<usize>, c: usize)
    requires
        sorted_by(ts, rank, v),
        forall|p: int| 0 <= p < v.len() ==> rank[(#[trigger] v[p]) as int] == c,
    ensures
        ct_sorted(ts, v),
{
    assert forall|p: int, q: int| 0 <= p < q < v.len() implies !ct_lt(
        ts[(#[trigger] v[q]) as int],
        ts[(#[trigger] v[p]) as int],
    ) by {
        assert(!key_lt(ts, rank, v[q], v[p]));
        assert(rank[v[q] as int] == c && rank[v[p] as int] == c);
    }
}

/// Entries of `idx` in bucket `c`, sorted by `created` then `id`.
fn bucket_sorted(ts: &Vec<Ticket>, bucket: &Vec<usize>, idx: &Vec<usize>, c: usize) -> (col: Vec<usize>)
    requires
        bucket@.len() == ts@.len(),
        in_range(idx@, ts@.len() as int),
        distinct(idx@),
    ensures
        forall|v: usize| col@.contains(v) <==> (idx@.contains(v) && bucket@[v as int] == c),
        distinct(col@),
        ct_sorted(ts@, col@),
{
    let sel = select(idx, bucket, c, ts.len());
    let col = sort_indices(ts, bucket, &sel);
    proof {
        assert forall|p: int| 0 <= p < col@.len() implies bucket@[(#[trigger] col@[p]) as int] == c by {
            assert(col@.contains(col@[p]));
            assert(sel@.contains(col@[p]));
        }
        lemma_sorted_same_rank(ts@, bucket@, col@, c);
    }
    col
}

#[allow(unused_variables)] // `n` is spec-only
/// Buckets `idx` by `bucket` into `cols` columns plus a trailing bucket `cols`,
/// each sorted by `created` then `id`.
fn layout(ts: &Vec<Ticket>, bucket: &Vec<usize>, idx: &Vec<usize>, cols: usize) -> (b: Board)
    requires
        bucket@.len() == ts@.len(),
        in_range(idx@, ts@.len() as int),
        distinct(idx@),
        forall|i: int| 0 <= i < bucket@.len() ==> (#[trigger] bucket@[i]) <= cols,
    ensures
        b.columns@.len() == cols,
        forall|c: int, v: usize|
            0 <= c < cols ==> (#[trigger] b.columns@[c]@.contains(v) <==> (idx@.contains(v) && bucket@[v as int] == c)),
        forall|v: usize| b.unknown@.contains(v) <==> (idx@.contains(v) && bucket@[v as int] == cols),
        forall|c: int| 0 <= c < cols ==> distinct((#[trigger] b.columns@[c])@) && ct_sorted(ts@, b.columns@[c]@),
        distinct(b.unknown@),
        ct_sorted(ts@, b.unknown@),
{
    let n = ts.len();
    let mut columns: Vec<Vec<usize>> = Vec::new();
    let mut c: usize = 0;
    while c < cols
        invariant
            n == ts@.len(),
            bucket@.len() == n,
            in_range(idx@, n as int),
            distinct(idx@),
            c <= cols,
            columns@.len() == c,
            forall|x: int, v: usize|
                0 <= x < c ==> (#[trigger] columns@[x]@.contains(v) <==> (idx@.contains(v) && bucket@[v as int] == x)),
            forall|x: int| 0 <= x < c ==> distinct((#[trigger] columns@[x])@) && ct_sorted(ts@, columns@[x]@),
        decreases cols - c,
    {
        let col = bucket_sorted(ts, bucket, idx, c);
        let ghost col_v = col@;
        columns.push(col);
        assert(columns@[c as int]@ == col_v);
        c = c + 1;
    }
    let unknown = bucket_sorted(ts, bucket, idx, cols);
    Board { columns, unknown }
}

/// TIX-21 / TIX-26: one column per schema status, in schema order, plus `?`.
pub fn board(ts: &Vec<Ticket>, s: &Schema, idx: &Vec<usize>) -> (b: Board)
    requires
        schema_rule1_names(*s),
        in_range(idx@, ts@.len() as int),
        distinct(idx@),
    ensures
        b.columns@.len() == s.statuses@.len(),
        // TIX-26: a ticket is in column c iff its status is c's name.
        forall|c: int, v: usize|
            0 <= c < s.statuses@.len() ==> (#[trigger] b.columns@[c]@.contains(v) <==> (idx@.contains(v)
                && ts@[v as int].status@ == s.statuses@[c].name@)),
        // TIX-26: a ticket is in `?` iff its status is not in the schema.
        forall|v: usize|
            #[trigger] b.unknown@.contains(v) <==> (idx@.contains(v) && !status_known(*s, ts@[v as int].status@)),
        // TIX-26: every input ticket is in exactly one column.
        forall|v: usize| #[trigger] idx@.contains(v) ==> (b.unknown@.contains(v) || exists|c: int|
            0 <= c < s.statuses@.len() && #[trigger] b.columns@[c]@.contains(v)),
        forall|c1: int, c2: int, v: usize|
            0 <= c1 < s.statuses@.len() && 0 <= c2 < s.statuses@.len() && #[trigger] b.columns@[c1]@.contains(v)
                && #[trigger] b.columns@[c2]@.contains(v) ==> c1 == c2,
        forall|c: int, v: usize|
            0 <= c < s.statuses@.len() && #[trigger] b.columns@[c]@.contains(v) ==> !b.unknown@.contains(v),
        // TIX-26: within a column, created ascending, ties by id.
        forall|c: int| 0 <= c < s.statuses@.len() ==> distinct((#[trigger] b.columns@[c])@) && ct_sorted(ts@, b.columns@[c]@),
        distinct(b.unknown@),
        ct_sorted(ts@, b.unknown@),
{
    let ranks = status_ranks(ts, s);
    let ns = s.statuses.len();
    proof {
        assert forall|i: int| 0 <= i < ranks@.len() implies (#[trigger] ranks@[i]) <= ns by {
            assert(status_rank_of(*s, ts@[i].status@, ranks@[i] as int));
        }
    }
    let b = layout(ts, &ranks, idx, ns);
    proof {
        assert forall|c: int, v: usize|
            0 <= c < ns implies (#[trigger] b.columns@[c]@.contains(v) <==> (idx@.contains(v)
                && ts@[v as int].status@ == s.statuses@[c].name@)) by {
            if idx@.contains(v) {
                let a = choose|a: int| 0 <= a < idx@.len() && idx@[a] == v;
                assert(v < ts@.len());
                assert(status_rank_of(*s, ts@[v as int].status@, ranks@[v as int] as int));
                lemma_rank_unique(*s, ts@[v as int].status@, ranks@[v as int] as int, c);
            }
        }
        assert forall|v: usize|
            #[trigger] b.unknown@.contains(v) <==> (idx@.contains(v) && !status_known(*s, ts@[v as int].status@)) by {
            if idx@.contains(v) {
                let a = choose|a: int| 0 <= a < idx@.len() && idx@[a] == v;
                let r = ranks@[v as int] as int;
                assert(status_rank_of(*s, ts@[v as int].status@, r));
                if r < ns {
                    assert(s.statuses@[r].name@ == ts@[v as int].status@);
                }
            }
        }
        assert forall|v: usize| #[trigger] idx@.contains(v) implies (b.unknown@.contains(v) || exists|c: int|
            0 <= c < s.statuses@.len() && #[trigger] b.columns@[c]@.contains(v)) by {
            let a = choose|a: int| 0 <= a < idx@.len() && idx@[a] == v;
            let r = ranks@[v as int] as int;
            if r < ns {
                assert(b.columns@[r]@.contains(v));
            }
        }
    }
    b
}

/// TIX-21 `--group` column index of a group name; 3 for anything else.
pub open spec fn group_col(g: Seq<char>) -> int {
    if g == "backlog"@ {
        0
    } else if g == "in_progress"@ {
        1
    } else if g == "completed"@ {
        2
    } else {
        3
    }
}

fn group_col_exec(g: &String) -> (r: usize)
    ensures
        r as int == group_col(g@),
{
    if str_is(g, "backlog") {
        0
    } else if str_is(g, "in_progress") {
        1
    } else if str_is(g, "completed") {
        2
    } else {
        3
    }
}

/// TIX-21 `--group`: three columns `backlog`, `in_progress`, `completed`, plus `?`.
pub fn board_groups(ts: &Vec<Ticket>, s: &Schema, idx: &Vec<usize>) -> (b: Board)
    requires
        schema_rule1_names(*s),
        schema_rule2(*s),
        in_range(idx@, ts@.len() as int),
        distinct(idx@),
    ensures
        b.columns@.len() == 3,
        forall|g: int, v: usize|
            0 <= g < 3 ==> (#[trigger] b.columns@[g]@.contains(v) <==> (idx@.contains(v) && exists|i: int|
                0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == ts@[v as int].status@
                    && group_col(s.statuses@[i].group@) == g)),
        forall|v: usize|
            #[trigger] b.unknown@.contains(v) <==> (idx@.contains(v) && !status_known(*s, ts@[v as int].status@)),
        forall|g: int| 0 <= g < 3 ==> distinct((#[trigger] b.columns@[g])@) && ct_sorted(ts@, b.columns@[g]@),
        distinct(b.unknown@),
        ct_sorted(ts@, b.unknown@),
{
    let ranks = status_ranks(ts, s);
    let st = &s.statuses;
    let ns = st.len();
    let mut gr: Vec<usize> = Vec::new();
    let mut i: usize = 0;
    while i < ranks.len()
        invariant
            st == &s.statuses,
            ns == st@.len(),
            ranks@.len() == ts@.len(),
            forall|x: int| 0 <= x < ts@.len() ==> status_rank_of(*s, (#[trigger] ts@[x]).status@, ranks@[x] as int),
            i <= ranks@.len(),
            gr@.len() == i,
            forall|x: int| 0 <= x < i ==> (#[trigger] gr@[x]) <= 3,
            forall|x: int|
                0 <= x < i ==> (#[trigger] gr@[x]) as int == if ranks@[x] < ns {
                    group_col(st@[ranks@[x] as int].group@)
                } else {
                    3
                },
        decreases ranks@.len() - i,
    {
        let r = ranks[i];
        let g = if r < ns {
            assert(status_rank_of(*s, ts@[i as int].status@, r as int));
            group_col_exec(&st[r].group)
        } else {
            3
        };
        gr.push(g);
        assert(gr@[i as int] == g);
        i = i + 1;
    }
    let b = layout(ts, &gr, idx, 3);
    proof {
        assert forall|g: int, v: usize|
            0 <= g < 3 implies (#[trigger] b.columns@[g]@.contains(v) <==> (idx@.contains(v) && exists|i: int|
                0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == ts@[v as int].status@
                    && group_col(s.statuses@[i].group@) == g)) by {
            if idx@.contains(v) {
                let a = choose|a: int| 0 <= a < idx@.len() && idx@[a] == v;
                let r = ranks@[v as int] as int;
                let name = ts@[v as int].status@;
                assert(status_rank_of(*s, name, r));
                if exists|i: int|
                    0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == name && group_col(
                        s.statuses@[i].group@,
                    ) == g {
                    let i = choose|i: int|
                        0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == name && group_col(
                            s.statuses@[i].group@,
                        ) == g;
                    lemma_rank_unique(*s, name, r, i);
                }
                if r < ns && gr@[v as int] == g {
                    assert(s.statuses@[r].name@ == name);
                }
            }
        }
        assert forall|v: usize|
            #[trigger] b.unknown@.contains(v) <==> (idx@.contains(v) && !status_known(*s, ts@[v as int].status@)) by {
            if idx@.contains(v) {
                let a = choose|a: int| 0 <= a < idx@.len() && idx@[a] == v;
                let r = ranks@[v as int] as int;
                let name = ts@[v as int].status@;
                assert(status_rank_of(*s, name, r));
                if r < ns {
                    assert(s.statuses@[r].name@ == name);
                    assert(valid_group(s.statuses@[r].group@));
                    assert(group_col(s.statuses@[r].group@) < 3);
                } else if status_known(*s, name) {
                    let i = choose|i: int| 0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == name;
                }
            }
        }
    }
    b
}

} // verus!
