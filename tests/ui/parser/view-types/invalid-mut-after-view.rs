// Test invalid usage of `mut` after view syntax

struct Point {
    x: i32,
    y: i32,
}

// Error: mut after view in function parameter
fn invalid_param(_p: &{x} mut Point) {
    //~^ ERROR: `mut` cannot come after the view
}

// Error: mut after view in method receiver
struct Container {
    value: i32,
}

impl Container {
    fn invalid_method(&mut {mut value} mut self) {
        //~^ ERROR: `mut` cannot come after the view
    }
}

fn main() {}
