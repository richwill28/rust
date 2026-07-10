// Test various mutability combinations in views
//@ check-pass

struct Point {
    x: i32,
    y: i32,
    z: i32,
}

// All immutable (no mut keywords)
fn all_immutable(p: &{x, y, z} Point) -> (&i32, &i32, &i32) {
    (&p.x, &p.y, &p.z)
}

// All mutable (mut before each field)
fn all_mutable(p: &mut {mut x, mut y, mut z} Point) -> (&mut i32, &mut i32, &mut i32) {
    (&mut p.x, &mut p.y, &mut p.z)
}

// Mixed mutability
fn mixed_first_mut(p: &mut {mut x, y, z} Point) -> (&mut i32, &i32, &i32) {
    (&mut p.x, &p.y, &p.z)
}

fn mixed_middle_mut(p: &mut {x, mut y, z} Point) -> (&i32, &mut i32, &i32) {
    (&p.x, &mut p.y, &p.z)
}

fn mixed_last_mut(p: &mut {x, y, mut z} Point) -> (&i32, &i32, &mut i32) {
    (&p.x, &p.y, &mut p.z)
}

fn mixed_two_mut(p: &mut {mut x, mut y, z} Point) -> (&mut i32, &mut i32, &i32) {
    (&mut p.x, &mut p.y, &p.z)
}

// Nested paths with mutability
struct Nested {
    point: Point,
}

fn nested_mut(n: &mut {mut point.x, point.y} Nested) -> (&mut i32, &i32) {
    (&mut n.point.x, &n.point.y)
}

fn main() {}
