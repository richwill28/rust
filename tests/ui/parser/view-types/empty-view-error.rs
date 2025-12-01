// Test that empty views produce an error

struct Point {
    x: i32,
    y: i32,
}

fn borrow_empty(_p: &{} Point) {
    //~^ ERROR empty view is not allowed
}

fn main() {}
