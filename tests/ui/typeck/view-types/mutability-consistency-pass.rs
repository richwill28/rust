// Test that mutability consistency is enforced correctly
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

// Immutable fields with immutable reference
fn immut_fields_immut_ref(p: &{x, y} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
}

// Mutable fields with mutable reference
fn mut_fields_mut_ref(p: &mut {mut x, mut y} Point) -> (&mut i32, &mut i32) {
    (&mut p.x, &mut p.y)
}

// Mixed: immutable fields with mutable reference (should be OK)
fn immut_fields_mut_ref(p: &mut {x, y} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
}

// Mixed: some mut, some immut fields with mutable reference
fn mixed_fields_mut_ref(p: &mut {x, mut y} Point) -> (&i32, &mut i32) {
    (&p.x, &mut p.y)
}

fn main() {}
