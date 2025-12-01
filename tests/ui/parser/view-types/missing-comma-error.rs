// Test missing comma between fields in view

struct Point {
    x: i32,
    y: i32,
    z: i32,
}

fn missing_comma(p: &{x y} Point) {
    //~^ ERROR expected `,` or `}` in view
}

fn missing_comma_three(p: &{x, y z} Point) {
    //~^ ERROR expected `,` or `}` in view
}

fn main() {}
