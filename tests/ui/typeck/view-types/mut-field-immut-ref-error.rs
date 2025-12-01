// Test error when view has mutable fields but reference is immutable

struct Point {
    x: i32,
    y: i32,
}

fn mut_field_in_immut_ref(p: &{mut x} Point) {
    //~^ ERROR view contains mutable fields but reference is not mutable
}

fn mixed_mutability_immut_ref(p: &{x, mut y} Point) {
    //~^ ERROR view contains mutable fields but reference is not mutable
}

fn all_mut_fields_immut_ref(p: &{mut x, mut y} Point) {
    //~^ ERROR view contains mutable fields but reference is not mutable
}

// This should be OK
fn mut_field_mut_ref(p: &mut {mut x} Point) {
}

fn main() {}
