// Test complex nested field paths in views
//@ check-pass

struct Deep {
    level1: Level1,
}

struct Level1 {
    level2: Level2,
}

struct Level2 {
    level3: Level3,
}

struct Level3 {
    value: i32,
}

// Deep nested path through multiple struct levels
fn borrow_deep(d: &{level1.level2.level3.value} Deep) -> &i32 {
    &d.level1.level2.level3.value
}

// Multiple nested paths
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

fn borrow_multiple_nested(o: &{inner.value, other.data} Outer) -> (&i32, &i32) {
    (&o.inner.value, &o.other.data)
}

fn main() {}
