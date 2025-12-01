// Test error when accessing nested field not in view

struct Outer {
    inner: Inner,
}

struct Inner {
    x: i32,
    y: i32,
}

fn access_wrong_nested(o: &{inner.x} Outer) -> &i32 {
    &o.inner.y
    //~^ ERROR field `inner.y` is not accessible through this view
}

fn access_parent_when_child_in_view(o: &{inner.x} Outer) -> &Inner {
    &o.inner
    //~^ ERROR field `inner` is not accessible through this view
}

fn main() {}
