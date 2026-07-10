// Test that view allows access to subfields of included fields
//@ check-pass

struct Outer {
    inner: Inner,
}

struct Inner {
    nested: Nested,
}

struct Nested {
    value: i32,
}

// View includes 'inner', so we can access inner.nested.value
fn view_allows_deeper_access(o: &{inner} Outer) -> &i32 {
    &o.inner.nested.value
}

// View includes 'inner.nested', so we can access inner.nested.value
fn view_on_nested_allows_subfield(o: &{inner.nested} Outer) -> &i32 {
    &o.inner.nested.value
}

fn main() {}
