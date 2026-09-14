//! TIX-11 ticket validity: spec predicate `valid_ticket` and `Ticket::validate`,
//! proved equivalent (TIX-24), with the error naming the first violated rule.

use crate::schema::*;
use crate::text::*;
use crate::types::*;
use vstd::prelude::*;

verus! {

pub open spec fn value_nonempty(v: Value) -> bool {
    match v {
        Value::Str(x) => x@.len() > 0,
        Value::List(xs) => xs@.len() > 0,
    }
}

pub open spec fn status_known(s: Schema, name: Seq<char>) -> bool {
    exists|i: int| 0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).name@ == name
}

/// The ticket carries field `name` with a non-empty value.
pub open spec fn has_nonempty(fields: Seq<FieldEntry>, name: Seq<char>) -> bool {
    exists|j: int| 0 <= j < fields.len() && (#[trigger] fields[j]).name@ == name && value_nonempty(
        fields[j].value,
    )
}

/// Entry `e` is declared by some schema field and has that field's type.
pub open spec fn declared_ok(fs: Seq<Field>, e: FieldEntry) -> bool {
    exists|i: int| 0 <= i < fs.len() && (#[trigger] fs[i]).name@ == e.name@ && value_ok(fs[i], e.value)
}

/// TIX-11 rule 1: well-formed ULID and non-empty title.
pub open spec fn ticket_rule1(t: Ticket) -> bool {
    valid_ulid(t.id@) && t.title@.len() > 0
}

/// TIX-11 rule 2: status names a schema status.
pub open spec fn ticket_rule2(t: Ticket, s: Schema) -> bool {
    status_known(s, t.status@)
}

/// TIX-11 rule 3: every required field is present and non-empty.
pub open spec fn ticket_rule3(t: Ticket, s: Schema) -> bool {
    forall|i: int|
        0 <= i < s.fields@.len() && (#[trigger] s.fields@[i]).required ==> has_nonempty(
            t.fields@,
            s.fields@[i].name@,
        )
}

/// TIX-11 rule 4: keys unique; every present field declared and well-typed.
pub open spec fn ticket_rule4(t: Ticket, s: Schema) -> bool {
    &&& forall|a: int, b: int|
        0 <= a < b < t.fields@.len() ==> (#[trigger] t.fields@[a]).name@
            != (#[trigger] t.fields@[b]).name@
    &&& forall|j: int| 0 <= j < t.fields@.len() ==> declared_ok(s.fields@, #[trigger] t.fields@[j])
}

pub open spec fn deliverable_ok(d: Deliverable) -> bool {
    d.label@.len() > 0 && d.reference@.len() > 0
}

/// TIX-11 rule 5: deliverables have non-empty label and ref; refs unique.
pub open spec fn ticket_rule5(t: Ticket) -> bool {
    &&& forall|k: int| 0 <= k < t.deliverables@.len() ==> deliverable_ok(#[trigger] t.deliverables@[k])
    &&& forall|a: int, b: int|
        0 <= a < b < t.deliverables@.len() ==> (#[trigger] t.deliverables@[a]).reference@
            != (#[trigger] t.deliverables@[b]).reference@
}

pub open spec fn ticket_rule(t: Ticket, s: Schema, k: int) -> bool {
    if k == 1 {
        ticket_rule1(t)
    } else if k == 2 {
        ticket_rule2(t, s)
    } else if k == 3 {
        ticket_rule3(t, s)
    } else if k == 4 {
        ticket_rule4(t, s)
    } else {
        ticket_rule5(t)
    }
}

/// Formal form of TIX-11.
pub open spec fn valid_ticket(t: Ticket, s: Schema) -> bool {
    &&& ticket_rule1(t)
    &&& ticket_rule2(t, s)
    &&& ticket_rule3(t, s)
    &&& ticket_rule4(t, s)
    &&& ticket_rule5(t)
}

/// `e` names the first TIX-11 rule that `t` violates.
pub open spec fn first_ticket_violation(t: Ticket, s: Schema, e: TicketError) -> bool {
    &&& 1 <= e.rule <= 5
    &&& !ticket_rule(t, s, e.rule as int)
    &&& forall|k: int| 1 <= k < e.rule ==> ticket_rule(t, s, k)
}

/// Validity depends only on the contents of a ticket, not on which `Vec`s hold them.
pub proof fn lemma_same_contents(a: Ticket, b: Ticket, s: Schema)
    requires
        a.id@ == b.id@,
        a.title@ == b.title@,
        a.status@ == b.status@,
        a.fields@ == b.fields@,
        a.deliverables@ == b.deliverables@,
    ensures
        forall|k: int| ticket_rule(a, s, k) == ticket_rule(b, s, k),
        forall|e: TicketError| first_ticket_violation(a, s, e) == first_ticket_violation(b, s, e),
        valid_ticket(a, s) == valid_ticket(b, s),
{
    assert(ticket_rule1(a) == ticket_rule1(b));
    assert(ticket_rule2(a, s) == ticket_rule2(b, s));
    assert(ticket_rule3(a, s) == ticket_rule3(b, s));
    assert(ticket_rule4(a, s) == ticket_rule4(b, s)) by {
        assert(a.fields@ == b.fields@);
    }
    assert(ticket_rule5(a) == ticket_rule5(b));
    assert forall|k: int| #[trigger] ticket_rule(a, s, k) == ticket_rule(b, s, k) by {
        if k == 1 {
        } else if k == 2 {
        } else if k == 3 {
        } else if k == 4 {
        } else {
        }
    }
    assert forall|e: TicketError| #[trigger] first_ticket_violation(a, s, e) == first_ticket_violation(
        b,
        s,
        e,
    ) by {
        assert(ticket_rule(a, s, e.rule as int) == ticket_rule(b, s, e.rule as int));
        if first_ticket_violation(a, s, e) {
            assert forall|k: int| 1 <= k < e.rule implies ticket_rule(b, s, k) by {
                assert(ticket_rule(a, s, k));
            }
        }
        if first_ticket_violation(b, s, e) {
            assert forall|k: int| 1 <= k < e.rule implies ticket_rule(a, s, k) by {
                assert(ticket_rule(b, s, k));
            }
        }
    }
}

pub fn check_status_known(s: &Schema, name: &String) -> (r: bool)
    ensures
        r == status_known(*s, name@),
{
    let st = &s.statuses;
    let mut i: usize = 0;
    while i < st.len()
        invariant
            st == &s.statuses,
            i <= st@.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] st@[j]).name@ != name@,
        decreases st@.len() - i,
    {
        if str_eq(&st[i].name, name) {
            return true;
        }
        i = i + 1;
    }
    false
}

fn check_value_nonempty(v: &Value) -> (r: bool)
    ensures
        r == value_nonempty(*v),
{
    match v {
        Value::Str(x) => x.as_str().unicode_len() > 0,
        Value::List(xs) => xs.len() > 0,
    }
}

fn check_has_nonempty(fields: &Vec<FieldEntry>, name: &String) -> (r: bool)
    ensures
        r == has_nonempty(fields@, name@),
{
    let mut j: usize = 0;
    while j < fields.len()
        invariant
            j <= fields@.len(),
            forall|x: int|
                0 <= x < j ==> !((#[trigger] fields@[x]).name@ == name@ && value_nonempty(
                    fields@[x].value,
                )),
        decreases fields@.len() - j,
    {
        if str_eq(&fields[j].name, name) && check_value_nonempty(&fields[j].value) {
            return true;
        }
        j = j + 1;
    }
    false
}

fn check_declared_ok(fs: &Vec<Field>, e: &FieldEntry) -> (r: bool)
    ensures
        r == declared_ok(fs@, *e),
{
    let mut i: usize = 0;
    while i < fs.len()
        invariant
            i <= fs@.len(),
            forall|x: int|
                0 <= x < i ==> !((#[trigger] fs@[x]).name@ == e.name@ && value_ok(fs@[x], e.value)),
        decreases fs@.len() - i,
    {
        if str_eq(&fs[i].name, &e.name) && check_value(&fs[i], &e.value) {
            return true;
        }
        i = i + 1;
    }
    false
}

impl Ticket {
    /// TIX-11 / TIX-24: `Ok` iff `valid_ticket`; `Err` names the first violated rule.
    pub fn validate(&self, s: &Schema) -> (r: Result<(), TicketError>)
        ensures
            r is Ok <==> valid_ticket(*self, *s),  // TIX-24
            r matches Err(e) ==> first_ticket_violation(*self, *s, e),  // TIX-11
    {
        // Rule 1.
        if !check_ulid(&self.id) || self.title.as_str().unicode_len() == 0 {
            return Err(TicketError { rule: 1, index: 0 });
        }
        // Rule 2.
        if !check_status_known(s, &self.status) {
            return Err(TicketError { rule: 2, index: 0 });
        }
        // Rule 3.
        let fs = &s.fields;
        let mut i: usize = 0;
        while i < fs.len()
            invariant
                fs == &s.fields,
                ticket_rule1(*self),
                ticket_rule2(*self, *s),
                i <= fs@.len(),
                forall|x: int|
                    0 <= x < i && (#[trigger] fs@[x]).required ==> has_nonempty(
                        self.fields@,
                        fs@[x].name@,
                    ),
            decreases fs@.len() - i,
        {
            if fs[i].required && !check_has_nonempty(&self.fields, &fs[i].name) {
                assert(!ticket_rule3(*self, *s)) by {
                    assert(fs@[i as int].required);
                }
                return Err(TicketError { rule: 3, index: i });
            }
            i = i + 1;
        }
        assert(ticket_rule3(*self, *s));
        // Rule 4.
        let tf = &self.fields;
        let mut b: usize = 0;
        while b < tf.len()
            invariant
                tf == &self.fields,
                ticket_rule1(*self),
                ticket_rule2(*self, *s),
                ticket_rule3(*self, *s),
                b <= tf@.len(),
                forall|x: int, y: int|
                    0 <= x < y < b ==> (#[trigger] tf@[x]).name@ != (#[trigger] tf@[y]).name@,
                forall|x: int| 0 <= x < b ==> declared_ok(s.fields@, #[trigger] tf@[x]),
            decreases tf@.len() - b,
        {
            if !check_declared_ok(&s.fields, &tf[b]) {
                return Err(TicketError { rule: 4, index: b });
            }
            let mut a: usize = 0;
            while a < b
                invariant
                    tf == &self.fields,
                    ticket_rule1(*self),
                    ticket_rule2(*self, *s),
                    ticket_rule3(*self, *s),
                    a <= b < tf@.len(),
                    forall|x: int, y: int|
                        0 <= x < y < b ==> (#[trigger] tf@[x]).name@ != (#[trigger] tf@[y]).name@,
                    forall|x: int| 0 <= x < a ==> (#[trigger] tf@[x]).name@ != tf@[b as int].name@,
                decreases b - a,
            {
                if str_eq(&tf[a].name, &tf[b].name) {
                    assert(!ticket_rule4(*self, *s)) by {
                        assert((tf@[a as int]).name@ == (tf@[b as int]).name@);
                    }
                    return Err(TicketError { rule: 4, index: b });
                }
                a = a + 1;
            }
            b = b + 1;
        }
        assert(ticket_rule4(*self, *s));
        // Rule 5.
        let ds = &self.deliverables;
        let mut b: usize = 0;
        while b < ds.len()
            invariant
                ds == &self.deliverables,
                ticket_rule1(*self),
                ticket_rule2(*self, *s),
                ticket_rule3(*self, *s),
                ticket_rule4(*self, *s),
                b <= ds@.len(),
                forall|x: int| 0 <= x < b ==> deliverable_ok(#[trigger] ds@[x]),
                forall|x: int, y: int|
                    0 <= x < y < b ==> (#[trigger] ds@[x]).reference@ != (#[trigger] ds@[y]).reference@,
            decreases ds@.len() - b,
        {
            if ds[b].label.as_str().unicode_len() == 0 || ds[b].reference.as_str().unicode_len() == 0 {
                assert(!deliverable_ok(ds@[b as int]));
                return Err(TicketError { rule: 5, index: b });
            }
            let mut a: usize = 0;
            while a < b
                invariant
                    ds == &self.deliverables,
                    ticket_rule1(*self),
                    ticket_rule2(*self, *s),
                    ticket_rule3(*self, *s),
                    ticket_rule4(*self, *s),
                    a <= b < ds@.len(),
                    forall|x: int, y: int|
                        0 <= x < y < b ==> (#[trigger] ds@[x]).reference@
                            != (#[trigger] ds@[y]).reference@,
                    forall|x: int| 0 <= x < a ==> (#[trigger] ds@[x]).reference@ != ds@[b as int].reference@,
                decreases b - a,
            {
                if str_eq(&ds[a].reference, &ds[b].reference) {
                    assert(!ticket_rule5(*self)) by {
                        assert((ds@[a as int]).reference@ == (ds@[b as int]).reference@);
                    }
                    return Err(TicketError { rule: 5, index: b });
                }
                a = a + 1;
            }
            b = b + 1;
        }
        Ok(())
    }
}

} // verus!
