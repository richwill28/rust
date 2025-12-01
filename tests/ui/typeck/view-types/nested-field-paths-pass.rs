// Test that valid nested field paths work correctly
//@ check-pass

struct Outer {
    inner: Inner,
    other: Other,
}

struct Inner {
    value: i32,
}

struct Other {
    data: i32,
}

fn nested_access(o: &{inner.value} Outer) -> &i32 {
    &o.inner.value
}

fn multiple_nested(o: &{inner.value, other.data} Outer) -> (&i32, &i32) {
    (&o.inner.value, &o.other.data)
}

fn main() {}
