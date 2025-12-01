// Test error when view contains duplicate fields

struct Point {
    x: i32,
    y: i32,
}

fn duplicate_field(p: &{x, x} Point) {
    //~^ ERROR view contains overlapping field paths
}

fn duplicate_among_many(p: &{x, y, x} Point) {
    //~^ ERROR view contains overlapping field paths
}

fn main() {}
