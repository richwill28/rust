// Test error when accessing field not included in view

struct Point {
    x: i32,
    y: i32,
}

fn access_outside_view(p: &{x} Point) -> &i32 {
    &p.y
    //~^ ERROR field `y` is not accessible through this view
}

fn access_multiple_outside(p: &{x} Point) -> (&i32, &i32) {
    (&p.x, &p.y)
    //~^ ERROR field `y` is not accessible through this view
}

fn main() {}
