// Test that valid field references in views pass type checking
//@ check-pass

struct Point {
    x: i32,
    y: i32,
}

fn single_field(p: &{x} Point) -> &i32 {
    &p.x
}

fn multiple_fields(p: &{x, y} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
}

fn main() {}
