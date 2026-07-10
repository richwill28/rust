// Test trailing commas in views
//@ check-pass

struct Point {
    x: i32,
    y: i32,
    z: i32,
}

// Trailing comma after single field
fn single_trailing(p: &{x,} Point) -> &i32 {
    &p.x
}

// Trailing comma after multiple fields
fn multiple_trailing(p: &{x, y,} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
}

// Trailing comma with mixed mutability
fn mixed_trailing(p: &mut {mut x, y,} Point) -> (&mut i32, &i32) {
    (&mut p.x, &p.y)
}

fn main() {}
