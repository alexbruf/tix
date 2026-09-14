//! TIX-25 write operations. Each builds the candidate ticket, validates it, and
//! returns it only if valid; an error names the first TIX-11 rule the
//! candidate violates. Inputs need not be valid, so `set` and `mv` can repair
//! tickets broken by a schema change (TIX-10).

#[allow(unused_imports)] // spec-only items
use crate::schema::*;
use crate::text::*;
#[allow(unused_imports)] // spec-only items
use crate::ticket::*;
use crate::types::*;
use vstd::prelude::*;

verus! {

/// One `KEY=VALUE` of `tix set`; `value: None` removes the field.
#[derive(Debug)]
pub struct Assignment {
    pub name: String,
    pub value: Option<Value>,
}

/// Why `detach` refused, besides validation.
#[derive(Debug, PartialEq, Eq)]
pub enum DetachError {
    NoMatch,
    Ambiguous,
    Invalid(TicketError),
}

/// Parts of a ticket every write keeps, plus `updated == now` (TIX-25).
pub open spec fn framed(t: Ticket, c: Ticket, now: u64) -> bool {
    &&& c.id == t.id
    &&& c.created == t.created
    &&& c.body == t.body
    &&& c.updated == now
}

// ---------------------------------------------------------------- transition

pub open spec fn transition_result(t: Ticket, c: Ticket, to: String, now: u64) -> bool {
    &&& framed(t, c, now)
    &&& c.status == to
    &&& c.title == t.title
    &&& c.fields == t.fields
    &&& c.deliverables == t.deliverables
}

/// `tix mv` (TIX-18, TIX-25).
pub fn transition(t: Ticket, s: &Schema, to: String, now: u64) -> (r: Result<Ticket, TicketError>)
    ensures
        r matches Ok(t2) ==> transition_result(t, t2, to, now) && valid_ticket(t2, *s),  // TIX-25
        r matches Err(e) ==> forall|c: Ticket|
            transition_result(t, c, to, now) ==> first_ticket_violation(c, *s, e),  // TIX-25
{
    let ghost to_g = to;
    let mut t2 = t;
    t2.status = to;
    t2.updated = now;
    match t2.validate(s) {
        Ok(()) => Ok(t2),
        Err(e) => {
            assert forall|c: Ticket| transition_result(t, c, to_g, now) implies first_ticket_violation(
                c,
                *s,
                e,
            ) by {
                assert(c == t2);
            }
            Err(e)
        },
    }
}

// -------------------------------------------------------------------- attach

pub open spec fn attach_result(t: Ticket, c: Ticket, d: Deliverable, now: u64) -> bool {
    &&& framed(t, c, now)
    &&& c.status == t.status
    &&& c.title == t.title
    &&& c.fields == t.fields
    &&& c.deliverables@ == t.deliverables@.push(d)
}

/// `tix attach` (TIX-20, TIX-25): appends `d`; a duplicate `ref` fails rule 5.
pub fn attach(t: Ticket, s: &Schema, d: Deliverable, now: u64) -> (r: Result<Ticket, TicketError>)
    ensures
        r matches Ok(t2) ==> attach_result(t, t2, d, now) && valid_ticket(t2, *s),  // TIX-25
        r matches Ok(t2) ==> t2.deliverables@.len() == t.deliverables@.len() + 1
            && t2.deliverables@.last() == d,  // TIX-25
        r matches Err(e) ==> forall|c: Ticket|
            attach_result(t, c, d, now) ==> first_ticket_violation(c, *s, e),  // TIX-25
{
    let ghost d_g = d;
    let mut t2 = t;
    t2.deliverables.push(d);
    t2.updated = now;
    match t2.validate(s) {
        Ok(()) => Ok(t2),
        Err(e) => {
            assert forall|c: Ticket| attach_result(t, c, d_g, now) implies first_ticket_violation(
                c,
                *s,
                e,
            ) by {
                lemma_same_contents(c, t2, *s);
            }
            Err(e)
        },
    }
}

// -------------------------------------------------------------------- detach

pub open spec fn deliverable_matches(d: Deliverable, arg: Seq<char>) -> bool {
    d.reference@ == arg || d.label@ == arg
}

pub open spec fn detach_result(t: Ticket, c: Ticket, k: int, now: u64) -> bool {
    &&& framed(t, c, now)
    &&& c.status == t.status
    &&& c.title == t.title
    &&& c.fields == t.fields
    &&& 0 <= k < t.deliverables@.len()
    &&& c.deliverables@ == t.deliverables@.remove(k)
}

/// `tix detach` (TIX-20, TIX-25): removes the single deliverable whose `ref`
/// or `label` equals `arg`.
pub fn detach(t: Ticket, s: &Schema, arg: &String, now: u64) -> (r: Result<Ticket, DetachError>)
    ensures
        r matches Ok(t2) ==> valid_ticket(t2, *s) && exists|k: int|
            detach_result(t, t2, k, now) && deliverable_matches(t.deliverables@[k], arg@),  // TIX-25
        r matches Ok(t2) ==> t2.deliverables@.len() == t.deliverables@.len() - 1,  // TIX-25
        r matches Ok(t2) ==> forall|i: int|
            0 <= i < t2.deliverables@.len() ==> !deliverable_matches(
                #[trigger] t2.deliverables@[i],
                arg@,
            ),  // TIX-25
        r matches Err(DetachError::NoMatch) ==> forall|i: int|
            0 <= i < t.deliverables@.len() ==> !deliverable_matches(
                #[trigger] t.deliverables@[i],
                arg@,
            ),
        r matches Err(DetachError::Ambiguous) ==> exists|i: int, j: int|
            0 <= i < j < t.deliverables@.len() && deliverable_matches(
                #[trigger] t.deliverables@[i],
                arg@,
            ) && deliverable_matches(#[trigger] t.deliverables@[j], arg@),
        r matches Err(DetachError::Invalid(e)) ==> exists|k: int|
            deliverable_matches(t.deliverables@[k], arg@) && forall|c: Ticket|
                detach_result(t, c, k, now) ==> first_ticket_violation(c, *s, e),  // TIX-25
{
    let ds = &t.deliverables;
    let n = ds.len();
    let mut found: usize = n;
    let mut i: usize = 0;
    while i < n
        invariant
            ds@ == t.deliverables@,
            n == ds@.len(),
            i <= n,
            found <= n,
            found == n ==> forall|x: int| 0 <= x < i ==> !deliverable_matches(#[trigger] ds@[x], arg@),
            found < n ==> found < i && deliverable_matches(ds@[found as int], arg@) && forall|x: int|
                0 <= x < i && x != found ==> !deliverable_matches(#[trigger] ds@[x], arg@),
        decreases n - i,
    {
        let m = str_eq(&ds[i].reference, arg) || str_eq(&ds[i].label, arg);
        if m {
            if found < n {
                assert(deliverable_matches(ds@[found as int], arg@));
                assert(deliverable_matches(ds@[i as int], arg@));
                return Err(DetachError::Ambiguous);
            }
            found = i;
        }
        i = i + 1;
    }
    if found == n {
        return Err(DetachError::NoMatch);
    }
    let ghost old_ds = t.deliverables@;
    let mut t2 = t;
    let _removed = t2.deliverables.remove(found);
    t2.updated = now;
    assert(t2.deliverables@ == old_ds.remove(found as int));
    assert forall|x: int| 0 <= x < t2.deliverables@.len() implies !deliverable_matches(
        #[trigger] t2.deliverables@[x],
        arg@,
    ) by {
        if x < found {
            assert(t2.deliverables@[x] == old_ds[x]);
        } else {
            assert(t2.deliverables@[x] == old_ds[x + 1]);
        }
    }
    assert(detach_result(t, t2, found as int, now));
    assert(deliverable_matches(t.deliverables@[found as int], arg@));
    match t2.validate(s) {
        Ok(()) => Ok(t2),
        Err(e) => {
            assert forall|c: Ticket| detach_result(t, c, found as int, now) implies first_ticket_violation(
                c,
                *s,
                e,
            ) by {
                lemma_same_contents(c, t2, *s);
            }
            Err(DetachError::Invalid(e))
        },
    }
}

// ---------------------------------------------------------------- new_ticket

/// `i` is the first status in group `backlog`.
pub open spec fn first_backlog(s: Schema, i: int) -> bool {
    &&& 0 <= i < s.statuses@.len()
    &&& s.statuses@[i].group@ == "backlog"@
    &&& forall|j: int| 0 <= j < i ==> (#[trigger] s.statuses@[j]).group@ != "backlog"@
}

pub open spec fn new_result(
    c: Ticket,
    s: Schema,
    id: String,
    title: String,
    status: Option<String>,
    fields: Seq<FieldEntry>,
    now: u64,
) -> bool {
    &&& c.id == id
    &&& c.title == title
    &&& c.created == now
    &&& c.updated == now
    &&& c.body@.len() == 0
    &&& c.deliverables@.len() == 0
    &&& c.fields@ == fields
    &&& match status {
        Some(st) => c.status == st,
        None => exists|i: int| first_backlog(s, i) && c.status@ == s.statuses@[i].name@,
    }
}

fn first_backlog_name(s: &Schema) -> (r: String)
    requires
        schema_rule3(*s),
    ensures
        exists|i: int| first_backlog(*s, i) && r@ == s.statuses@[i].name@,
{
    let st = &s.statuses;
    let mut i: usize = 0;
    while i < st.len()
        invariant
            st == &s.statuses,
            schema_rule3(*s),
            i <= st@.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] st@[j]).group@ != "backlog"@,
        decreases st@.len() - i,
    {
        if str_is(&st[i].group, "backlog") {
            assert(first_backlog(*s, i as int));
            return st[i].name.clone();
        }
        i = i + 1;
    }
    assert(false);
    String::new()
}

/// `tix new` (TIX-15, TIX-25). `status: None` takes the first backlog status.
pub fn new_ticket(
    s: &Schema,
    id: String,
    title: String,
    status: Option<String>,
    fields: Vec<FieldEntry>,
    now: u64,
) -> (r: Result<Ticket, TicketError>)
    requires
        valid_schema(*s),
    ensures
        r matches Ok(t2) ==> new_result(t2, *s, id, title, status, fields@, now) && valid_ticket(
            t2,
            *s,
        ),  // TIX-25
        r matches Err(e) ==> forall|c: Ticket|
            new_result(c, *s, id, title, status, fields@, now) ==> first_ticket_violation(
                c,
                *s,
                e,
            ),  // TIX-25
{
    let ghost (id_g, title_g, status_g, fields_g) = (id, title, status, fields@);
    let st = match status {
        Some(x) => x,
        None => first_backlog_name(s),
    };
    let t2 = Ticket {
        id,
        title,
        status: st,
        created: now,
        updated: now,
        deliverables: Vec::new(),
        fields,
        body: String::new(),
    };
    assert(new_result(t2, *s, id_g, title_g, status_g, fields_g, now));
    match t2.validate(s) {
        Ok(()) => Ok(t2),
        Err(e) => {
            assert forall|c: Ticket|
                new_result(c, *s, id_g, title_g, status_g, fields_g, now) implies first_ticket_violation(
                c,
                *s,
                e,
            ) by {
                if status_g is None {
                    let i = choose|i: int| first_backlog(*s, i) && c.status@ == s.statuses@[i].name@;
                    let j = choose|j: int| first_backlog(*s, j) && t2.status@ == s.statuses@[j].name@;
                    assert(i == j) by {
                        if i < j {
                            assert(s.statuses@[i].group@ == "backlog"@);
                        } else if j < i {
                            assert(s.statuses@[j].group@ == "backlog"@);
                        }
                    }
                }
                assert(c.deliverables@ =~= t2.deliverables@);
                lemma_same_contents(c, t2, *s);
            }
            Err(e)
        },
    }
}

} // verus!

verus! {

// ---------------------------------------------------------------- set_fields

pub open spec fn remove_named(fs: Seq<FieldEntry>, n: Seq<char>) -> Seq<FieldEntry>
    decreases fs.len(),
{
    if fs.len() == 0 {
        fs
    } else if fs[0].name@ == n {
        remove_named(fs.drop_first(), n)
    } else {
        seq![fs[0]] + remove_named(fs.drop_first(), n)
    }
}

pub open spec fn apply_one(fs: Seq<FieldEntry>, a: Assignment) -> Seq<FieldEntry> {
    match a.value {
        Some(v) => remove_named(fs, a.name@).push(FieldEntry { name: a.name, value: v }),
        None => remove_named(fs, a.name@),
    }
}

pub open spec fn apply_all(fs: Seq<FieldEntry>, asg: Seq<Assignment>) -> Seq<FieldEntry>
    decreases asg.len(),
{
    if asg.len() == 0 {
        fs
    } else {
        apply_one(apply_all(fs, asg.drop_last()), asg.last())
    }
}

pub open spec fn names_distinct(asg: Seq<Assignment>) -> bool {
    forall|i: int, j: int| 0 <= i < j < asg.len() ==> (#[trigger] asg[i]).name@ != (#[trigger] asg[j]).name@
}

pub open spec fn named(asg: Seq<Assignment>, n: Seq<char>) -> bool {
    exists|k: int| 0 <= k < asg.len() && (#[trigger] asg[k]).name@ == n
}

pub open spec fn set_result(
    t: Ticket,
    c: Ticket,
    title: Option<String>,
    asg: Seq<Assignment>,
    now: u64,
) -> bool {
    &&& framed(t, c, now)
    &&& c.status == t.status
    &&& c.deliverables == t.deliverables
    &&& match title {
        Some(x) => c.title == x,
        None => c.title == t.title,
    }
    &&& c.fields@ == apply_all(t.fields@, asg)
}

proof fn lemma_push_contains(s: Seq<FieldEntry>, x: FieldEntry)
    ensures
        forall|e: FieldEntry| #[trigger] s.push(x).contains(e) <==> (s.contains(e) || e == x),
{
    assert forall|e: FieldEntry| #[trigger] s.push(x).contains(e) <==> (s.contains(e) || e == x) by {
        if s.contains(e) {
            let i = choose|i: int| 0 <= i < s.len() && s[i] == e;
            assert(s.push(x)[i] == e);
        }
        if e == x {
            assert(s.push(x)[s.len() as int] == e);
        }
        if s.push(x).contains(e) {
            let i = choose|i: int| 0 <= i < s.push(x).len() && s.push(x)[i] == e;
            if i < s.len() {
                assert(s[i] == e);
            }
        }
    }
}

proof fn lemma_remove_named(fs: Seq<FieldEntry>, n: Seq<char>)
    ensures
        forall|e: FieldEntry|
            #[trigger] remove_named(fs, n).contains(e) <==> (fs.contains(e) && e.name@ != n),
    decreases fs.len(),
{
    if fs.len() > 0 {
        let rest = fs.drop_first();
        lemma_remove_named(rest, n);
        assert forall|e: FieldEntry|
            #[trigger] remove_named(fs, n).contains(e) <==> (fs.contains(e) && e.name@ != n) by {
            let r = remove_named(rest, n);
            if fs.contains(e) {
                let i = choose|i: int| 0 <= i < fs.len() && fs[i] == e;
                if i > 0 {
                    assert(rest[i - 1] == e);
                }
            }
            if rest.contains(e) {
                let i = choose|i: int| 0 <= i < rest.len() && rest[i] == e;
                assert(fs[i + 1] == e);
            }
            if fs[0].name@ != n {
                let full = seq![fs[0]] + r;
                if full.contains(e) {
                    let i = choose|i: int| 0 <= i < full.len() && full[i] == e;
                    if i > 0 {
                        assert(r[i - 1] == e);
                    }
                }
                if r.contains(e) {
                    let i = choose|i: int| 0 <= i < r.len() && r[i] == e;
                    assert(full[i + 1] == e);
                }
                assert(full[0] == fs[0]);
            }
        }
    }
}

proof fn lemma_apply_all(fs: Seq<FieldEntry>, asg: Seq<Assignment>)
    requires
        names_distinct(asg),
    ensures
        forall|e: FieldEntry|
            !named(asg, e.name@) ==> (#[trigger] apply_all(fs, asg).contains(e) <==> fs.contains(e)),
        forall|e: FieldEntry|
            #[trigger] apply_all(fs, asg).contains(e) && named(asg, e.name@) ==> exists|k: int|
                0 <= k < asg.len() && (#[trigger] asg[k]).name@ == e.name@ && asg[k].value == Some(
                    e.value,
                ),
        forall|k: int|
            0 <= k < asg.len() && (#[trigger] asg[k]).value is Some ==> apply_all(fs, asg).contains(
                FieldEntry { name: asg[k].name, value: asg[k].value->Some_0 },
            ),
    decreases asg.len(),
{
    if asg.len() > 0 {
        let p = asg.drop_last();
        let a = asg.last();
        assert(names_distinct(p));
        lemma_apply_all(fs, p);
        let r0 = apply_all(fs, p);
        let rm = remove_named(r0, a.name@);
        lemma_remove_named(r0, a.name@);
        let r = apply_all(fs, asg);
        assert(r == apply_one(r0, a));
        match a.value {
            Some(v) => {
                lemma_push_contains(rm, FieldEntry { name: a.name, value: v });
            },
            None => {},
        }
        assert forall|k: int| 0 <= k < p.len() implies (#[trigger] p[k]).name@ != a.name@ by {
            assert(asg[k] == p[k]);
        }
        // Property 1.
        assert forall|e: FieldEntry| !named(asg, e.name@) implies (#[trigger] r.contains(e)
            <==> fs.contains(e)) by {
            assert(e.name@ != a.name@);
            if named(p, e.name@) {
                let k = choose|k: int| 0 <= k < p.len() && (#[trigger] p[k]).name@ == e.name@;
                assert(asg[k] == p[k]);
            }
        }
        // Property 2.
        assert forall|e: FieldEntry| #[trigger] r.contains(e) && named(asg, e.name@) implies exists|k: int|
            0 <= k < asg.len() && (#[trigger] asg[k]).name@ == e.name@ && asg[k].value == Some(
                e.value,
            ) by {
            let last = asg.len() - 1;
            assert(asg[last] == a);
            if e.name@ == a.name@ {
                assert(!rm.contains(e));
            } else {
                assert(rm.contains(e));
                assert(r0.contains(e));
                let kk = choose|kk: int| 0 <= kk < asg.len() && (#[trigger] asg[kk]).name@ == e.name@;
                assert(kk != last);
                assert(p[kk] == asg[kk]);
                assert(named(p, e.name@));
                let k = choose|k: int|
                    0 <= k < p.len() && (#[trigger] p[k]).name@ == e.name@ && p[k].value == Some(e.value);
                assert(asg[k] == p[k]);
            }
        }
        // Property 3.
        assert forall|k: int| 0 <= k < asg.len() && (#[trigger] asg[k]).value is Some implies r.contains(
            FieldEntry { name: asg[k].name, value: asg[k].value->Some_0 },
        ) by {
            let ent = FieldEntry { name: asg[k].name, value: asg[k].value->Some_0 };
            if k == asg.len() - 1 {
                assert(asg[k] == a);
            } else {
                assert(p[k] == asg[k]);
                assert(r0.contains(ent));
                assert(rm.contains(ent));
            }
        }
    }
}

fn remove_named_exec(fs: &mut Vec<FieldEntry>, n: &String)
    ensures
        final(fs)@ == remove_named(old(fs)@, n@),
{
    let ghost o = old(fs)@;
    let len = fs.len();
    let mut i: usize = len;
    assert(o.subrange(len as int, len as int).len() == 0);
    assert(fs@ =~= o.subrange(0, len as int) + remove_named(o.subrange(len as int, len as int), n@));
    while i > 0
        invariant
            len == o.len(),
            i <= len,
            fs@ == o.subrange(0, i as int) + remove_named(o.subrange(i as int, len as int), n@),
        decreases i,
    {
        let j = i - 1;
        let ghost tail = o.subrange(j as int, len as int);
        let ghost rest = remove_named(o.subrange(i as int, len as int), n@);
        assert(tail.drop_first() =~= o.subrange(i as int, len as int));
        assert(tail[0] == o[j as int]);
        assert(fs@[j as int] == o[j as int]);
        if str_eq(&fs[j].name, n) {
            fs.remove(j);
            assert(fs@ =~= o.subrange(0, j as int) + remove_named(tail, n@));
        } else {
            assert(fs@ =~= o.subrange(0, j as int) + remove_named(tail, n@));
        }
        i = j;
    }
    assert(o.subrange(0, len as int) =~= o);
    assert(fs@ =~= remove_named(o, n@));
}

/// `tix set` (TIX-19, TIX-25). `title: None` leaves the title alone;
/// an assignment with `value: None` removes that field.
pub fn set_fields(
    t: Ticket,
    s: &Schema,
    title: Option<String>,
    assigns: Vec<Assignment>,
    now: u64,
) -> (r: Result<Ticket, TicketError>)
    requires
        names_distinct(assigns@),
    ensures
        r matches Ok(t2) ==> set_result(t, t2, title, assigns@, now) && valid_ticket(t2, *s),  // TIX-25
        // TIX-25: fields not named are unchanged.
        r matches Ok(t2) ==> forall|e: FieldEntry|
            !named(assigns@, e.name@) ==> (#[trigger] t2.fields@.contains(e) <==> t.fields@.contains(e)),
        // TIX-19: named fields hold exactly the assigned value, or are absent when removed.
        r matches Ok(t2) ==> forall|e: FieldEntry|
            #[trigger] t2.fields@.contains(e) && named(assigns@, e.name@) ==> exists|k: int|
                0 <= k < assigns@.len() && (#[trigger] assigns@[k]).name@ == e.name@
                    && assigns@[k].value == Some(e.value),
        r matches Ok(t2) ==> forall|k: int|
            0 <= k < assigns@.len() && (#[trigger] assigns@[k]).value is Some ==> t2.fields@.contains(
                FieldEntry { name: assigns@[k].name, value: assigns@[k].value->Some_0 },
            ),
        r matches Err(e) ==> forall|c: Ticket|
            set_result(t, c, title, assigns@, now) ==> first_ticket_violation(c, *s, e),  // TIX-25
{
    let ghost orig = assigns@;
    let ghost title_g = title;
    let ghost t_fields = t.fields@;
    let n = assigns.len();
    let mut rest = assigns;
    let mut t2 = t;
    let mut k: usize = 0;
    while k < n
        invariant
            n == orig.len(),
            k <= n,
            rest@ == orig.subrange(k as int, n as int),
            t2.fields@ == apply_all(t_fields, orig.subrange(0, k as int)),
            t2.id == t.id,
            t2.created == t.created,
            t2.body == t.body,
            t2.status == t.status,
            t2.title == t.title,
            t2.deliverables == t.deliverables,
        decreases n - k,
    {
        let a = rest.remove(0);
        proof {
            let pre = orig.subrange(0, k as int + 1);
            assert(pre.drop_last() =~= orig.subrange(0, k as int));
            assert(pre.last() == orig[k as int]);
            assert(a == orig[k as int]);
        }
        remove_named_exec(&mut t2.fields, &a.name);
        match a.value {
            Some(v) => {
                t2.fields.push(FieldEntry { name: a.name, value: v });
            },
            None => {},
        }
        assert(rest@ =~= orig.subrange(k as int + 1, n as int));
        k = k + 1;
    }
    assert(orig.subrange(0, n as int) =~= orig);
    match title {
        Some(x) => {
            t2.title = x;
        },
        None => {},
    }
    t2.updated = now;
    assert(set_result(t, t2, title_g, orig, now));
    proof {
        lemma_apply_all(t_fields, orig);
    }
    match t2.validate(s) {
        Ok(()) => Ok(t2),
        Err(e) => {
            assert forall|c: Ticket| set_result(t, c, title_g, orig, now) implies first_ticket_violation(
                c,
                *s,
                e,
            ) by {
                lemma_same_contents(c, t2, *s);
            }
            Err(e)
        },
    }
}

} // verus!
