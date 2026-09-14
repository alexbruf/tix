//! TIX-9 schema validity: spec predicate `valid_schema` and `Schema::validate`,
//! proved equivalent (TIX-24), with the error naming the first violated rule.

use crate::text::*;
use crate::types::*;
use vstd::prelude::*;

verus! {

pub open spec fn strings_distinct(v: Seq<String>) -> bool {
    forall|a: int, b: int| 0 <= a < b < v.len() ==> (#[trigger] v[a])@ != (#[trigger] v[b])@
}

pub open spec fn seq_contains_str(v: Seq<String>, x: Seq<char>) -> bool {
    exists|i: int| 0 <= i < v.len() && (#[trigger] v[i])@ == x
}

/// A value has the declared type of `f` (TIX-11 rule 4, also TIX-9 rule 6).
pub open spec fn value_ok(f: Field, v: Value) -> bool {
    match v {
        Value::Str(x) => match f.ty {
            FieldType::Str => x@.len() > 0,
            FieldType::Enum => seq_contains_str(f.values@, x@),
            FieldType::Date => valid_date(x@),
            FieldType::List => false,
        },
        Value::List(xs) => f.ty == FieldType::List && forall|i: int|
            0 <= i < xs@.len() ==> (#[trigger] xs@[i])@.len() > 0,
    }
}

/// TIX-9 rule 1: statuses non-empty, names unique.
pub open spec fn schema_rule1(s: Schema) -> bool {
    &&& s.statuses@.len() > 0
    &&& forall|a: int, b: int|
        0 <= a < b < s.statuses@.len() ==> (#[trigger] s.statuses@[a]).name@
            != (#[trigger] s.statuses@[b]).name@
}

/// TIX-9 rule 2: every group is one of the three fixed groups.
pub open spec fn schema_rule2(s: Schema) -> bool {
    forall|i: int| 0 <= i < s.statuses@.len() ==> valid_group((#[trigger] s.statuses@[i]).group@)
}

/// TIX-9 rule 3: some status is in group `backlog`.
pub open spec fn schema_rule3(s: Schema) -> bool {
    exists|i: int| 0 <= i < s.statuses@.len() && (#[trigger] s.statuses@[i]).group@ == "backlog"@
}

/// TIX-9 rule 4: field names unique, lexically valid, not built-in.
pub open spec fn schema_rule4(s: Schema) -> bool {
    &&& forall|a: int, b: int|
        0 <= a < b < s.fields@.len() ==> (#[trigger] s.fields@[a]).name@
            != (#[trigger] s.fields@[b]).name@
    &&& forall|i: int| 0 <= i < s.fields@.len() ==> valid_field_name((#[trigger] s.fields@[i]).name@)
}

pub open spec fn enum_ok(f: Field) -> bool {
    f.ty == FieldType::Enum ==> (f.values@.len() > 0 && strings_distinct(f.values@))
}

/// TIX-9 rule 5: enum fields have at least one value, values unique.
pub open spec fn schema_rule5(s: Schema) -> bool {
    forall|i: int| 0 <= i < s.fields@.len() ==> enum_ok(#[trigger] s.fields@[i])
}

pub open spec fn default_ok(f: Field) -> bool {
    match f.default {
        Some(v) => value_ok(f, v),
        None => true,
    }
}

/// TIX-9 rule 6: every default is a valid value for its field.
pub open spec fn schema_rule6(s: Schema) -> bool {
    forall|i: int| 0 <= i < s.fields@.len() ==> default_ok(#[trigger] s.fields@[i])
}

pub open spec fn schema_rule(s: Schema, k: int) -> bool {
    if k == 1 {
        schema_rule1(s)
    } else if k == 2 {
        schema_rule2(s)
    } else if k == 3 {
        schema_rule3(s)
    } else if k == 4 {
        schema_rule4(s)
    } else if k == 5 {
        schema_rule5(s)
    } else {
        schema_rule6(s)
    }
}

/// Formal form of TIX-9.
pub open spec fn valid_schema(s: Schema) -> bool {
    &&& schema_rule1(s)
    &&& schema_rule2(s)
    &&& schema_rule3(s)
    &&& schema_rule4(s)
    &&& schema_rule5(s)
    &&& schema_rule6(s)
}

/// `e` names the first TIX-9 rule that `s` violates.
pub open spec fn first_schema_violation(s: Schema, e: SchemaError) -> bool {
    &&& 1 <= e.rule <= 6
    &&& !schema_rule(s, e.rule as int)
    &&& forall|k: int| 1 <= k < e.rule ==> schema_rule(s, k)
}

pub fn vec_contains_str(v: &Vec<String>, x: &String) -> (r: bool)
    ensures
        r == seq_contains_str(v@, x@),
{
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v@.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] v@[j])@ != x@,
        decreases v@.len() - i,
    {
        if str_eq(&v[i], x) {
            return true;
        }
        i = i + 1;
    }
    false
}

pub fn check_distinct(v: &Vec<String>) -> (r: bool)
    ensures
        r == strings_distinct(v@),
{
    let n = v.len();
    let mut b: usize = 0;
    while b < n
        invariant
            n == v@.len(),
            b <= n,
            forall|x: int, y: int| 0 <= x < y < b ==> (#[trigger] v@[x])@ != (#[trigger] v@[y])@,
        decreases n - b,
    {
        let mut a: usize = 0;
        while a < b
            invariant
                n == v@.len(),
                a <= b < n,
                forall|x: int, y: int| 0 <= x < y < b ==> (#[trigger] v@[x])@ != (#[trigger] v@[y])@,
                forall|x: int| 0 <= x < a ==> (#[trigger] v@[x])@ != v@[b as int]@,
            decreases b - a,
        {
            if str_eq(&v[a], &v[b]) {
                return false;
            }
            a = a + 1;
        }
        b = b + 1;
    }
    true
}

pub fn all_nonempty(xs: &Vec<String>) -> (r: bool)
    ensures
        r == forall|i: int| 0 <= i < xs@.len() ==> (#[trigger] xs@[i])@.len() > 0,
{
    let mut i: usize = 0;
    while i < xs.len()
        invariant
            i <= xs@.len(),
            forall|j: int| 0 <= j < i ==> (#[trigger] xs@[j])@.len() > 0,
        decreases xs@.len() - i,
    {
        if xs[i].as_str().unicode_len() == 0 {
            assert((xs@[i as int])@.len() == 0);
            return false;
        }
        i = i + 1;
    }
    true
}

pub fn check_value(f: &Field, v: &Value) -> (r: bool)
    ensures
        r == value_ok(*f, *v),
{
    match v {
        Value::Str(x) => match f.ty {
            FieldType::Str => x.as_str().unicode_len() > 0,
            FieldType::Enum => vec_contains_str(&f.values, x),
            FieldType::Date => check_date(x),
            FieldType::List => false,
        },
        Value::List(xs) => match f.ty {
            FieldType::List => all_nonempty(xs),
            _ => false,
        },
    }
}

impl Schema {
    /// TIX-9 / TIX-24: `Ok` iff `valid_schema`; `Err` names the first violated rule.
    pub fn validate(&self) -> (r: Result<(), SchemaError>)
        ensures
            r is Ok <==> valid_schema(*self),  // TIX-24
            r matches Err(e) ==> first_schema_violation(*self, e),  // TIX-9
    {
        let st = &self.statuses;
        let fs = &self.fields;
        let ns = st.len();
        let nf = fs.len();

        // Rule 1.
        if ns == 0 {
            return Err(SchemaError { rule: 1, index: 0 });
        }
        let mut b: usize = 0;
        while b < ns
            invariant
                ns == st@.len(),
                st == &self.statuses,
                b <= ns,
                forall|x: int, y: int|
                    0 <= x < y < b ==> (#[trigger] st@[x]).name@ != (#[trigger] st@[y]).name@,
            decreases ns - b,
        {
            let mut a: usize = 0;
            while a < b
                invariant
                    ns == st@.len(),
                    st == &self.statuses,
                    a <= b < ns,
                    forall|x: int, y: int|
                        0 <= x < y < b ==> (#[trigger] st@[x]).name@ != (#[trigger] st@[y]).name@,
                    forall|x: int| 0 <= x < a ==> (#[trigger] st@[x]).name@ != st@[b as int].name@,
                decreases b - a,
            {
                if str_eq(&st[a].name, &st[b].name) {
                    assert(!schema_rule1(*self)) by {
                        assert((st@[a as int]).name@ == (st@[b as int]).name@);
                    }
                    return Err(SchemaError { rule: 1, index: b });
                }
                a = a + 1;
            }
            b = b + 1;
        }
        assert(schema_rule1(*self));

        // Rule 2.
        let mut i: usize = 0;
        while i < ns
            invariant
                ns == st@.len(),
                st == &self.statuses,
                schema_rule1(*self),
                i <= ns,
                forall|j: int| 0 <= j < i ==> valid_group((#[trigger] st@[j]).group@),
            decreases ns - i,
        {
            if !check_group(&st[i].group) {
                return Err(SchemaError { rule: 2, index: i });
            }
            i = i + 1;
        }
        assert(schema_rule2(*self));

        // Rule 3.
        let mut i: usize = 0;
        let mut found = false;
        while i < ns
            invariant
                ns == st@.len(),
                st == &self.statuses,
                i <= ns,
                !found ==> forall|j: int| 0 <= j < i ==> (#[trigger] st@[j]).group@ != "backlog"@,
                found ==> schema_rule3(*self),
            decreases ns - i,
        {
            if str_is(&st[i].group, "backlog") {
                found = true;
            }
            i = i + 1;
        }
        if !found {
            return Err(SchemaError { rule: 3, index: 0 });
        }

        // Rule 4.
        let mut b: usize = 0;
        while b < nf
            invariant
                nf == fs@.len(),
                fs == &self.fields,
                schema_rule1(*self),
                schema_rule2(*self),
                schema_rule3(*self),
                b <= nf,
                forall|x: int, y: int|
                    0 <= x < y < b ==> (#[trigger] fs@[x]).name@ != (#[trigger] fs@[y]).name@,
                forall|x: int| 0 <= x < b ==> valid_field_name((#[trigger] fs@[x]).name@),
            decreases nf - b,
        {
            if !check_field_name(&fs[b].name) {
                return Err(SchemaError { rule: 4, index: b });
            }
            let mut a: usize = 0;
            while a < b
                invariant
                    nf == fs@.len(),
                    fs == &self.fields,
                    schema_rule1(*self),
                    schema_rule2(*self),
                    schema_rule3(*self),
                    a <= b < nf,
                    forall|x: int, y: int|
                        0 <= x < y < b ==> (#[trigger] fs@[x]).name@ != (#[trigger] fs@[y]).name@,
                    forall|x: int| 0 <= x < a ==> (#[trigger] fs@[x]).name@ != fs@[b as int].name@,
                decreases b - a,
            {
                if str_eq(&fs[a].name, &fs[b].name) {
                    assert(!schema_rule4(*self)) by {
                        assert((fs@[a as int]).name@ == (fs@[b as int]).name@);
                    }
                    return Err(SchemaError { rule: 4, index: b });
                }
                a = a + 1;
            }
            b = b + 1;
        }
        assert(schema_rule4(*self));

        // Rule 5.
        let mut i: usize = 0;
        while i < nf
            invariant
                nf == fs@.len(),
                fs == &self.fields,
                schema_rule1(*self),
                schema_rule2(*self),
                schema_rule3(*self),
                schema_rule4(*self),
                i <= nf,
                forall|j: int| 0 <= j < i ==> enum_ok(#[trigger] fs@[j]),
            decreases nf - i,
        {
            match fs[i].ty {
                FieldType::Enum => {
                    if fs[i].values.len() == 0 || !check_distinct(&fs[i].values) {
                        assert(!enum_ok(fs@[i as int]));
                        return Err(SchemaError { rule: 5, index: i });
                    }
                },
                _ => {},
            }
            i = i + 1;
        }
        assert(schema_rule5(*self));

        // Rule 6.
        let mut i: usize = 0;
        while i < nf
            invariant
                nf == fs@.len(),
                fs == &self.fields,
                schema_rule1(*self),
                schema_rule2(*self),
                schema_rule3(*self),
                schema_rule4(*self),
                schema_rule5(*self),
                i <= nf,
                forall|j: int| 0 <= j < i ==> default_ok(#[trigger] fs@[j]),
            decreases nf - i,
        {
            match &fs[i].default {
                Some(v) => {
                    if !check_value(&fs[i], v) {
                        return Err(SchemaError { rule: 6, index: i });
                    }
                },
                None => {},
            }
            i = i + 1;
        }
        Ok(())
    }
}

} // verus!
