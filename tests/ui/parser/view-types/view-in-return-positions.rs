// Test view syntax in return type positions
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

// View in return type
fn returns_view_ref(p: &Point) -> &{x} Point {
    p
}

// View in function pointer return type
fn returns_fn() -> fn(&Point) -> &{x} Point {
    returns_view_ref
}

// View in closure return type
fn closure_returns_view() -> impl Fn(&Point) -> &{x} Point {
    |p| p
}

// View with lifetime in return type
fn returns_view_with_lifetime<'a>(p: &'a Point) -> &'a {x} Point {
    p
}

fn main() {}
