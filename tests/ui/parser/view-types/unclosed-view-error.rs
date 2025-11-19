// Test unclosed view brace error

struct Point {
    x: i32,
    y: i32,
}

fn missing_close(p: &{x Point) {
    //~^ ERROR mismatched closing delimiter: `)`
}

fn missing_close_multiple(p: &{x, y Point) {
    //~^ ERROR mismatched closing delimiter: `)`
}

fn main() {}
