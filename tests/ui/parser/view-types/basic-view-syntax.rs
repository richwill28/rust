// Test basic valid view syntax
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

struct Nested {
    point: Point,
}

// Basic single field view
fn borrow_x(p: &{x} Point) -> &i32 {
    &p.x
}

// Multiple fields
fn borrow_xy(p: &{x, y} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
}

// Mutable view
fn borrow_x_mut(p: &mut {mut x} Point) -> &mut i32 {
    &mut p.x
}

// Mixed mutability in view
fn borrow_xy_mixed(p: &mut {mut x, y} Point) -> (&mut i32, &i32) {
    (&mut p.x, &p.y)
}

// View with lifetime
fn borrow_with_lifetime<'a>(p: &'a {x} Point) -> &'a i32 {
    &p.x
}

// Nested field path
fn borrow_nested(n: &{point.x} Nested) -> &i32 {
    &n.point.x
}

fn main() {}
