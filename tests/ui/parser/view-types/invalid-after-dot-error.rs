// Test errors when invalid tokens appear after dot in nested field paths

struct Nested {
    point: Point,
}

struct Point {
    x: i32,
    y: i32,
}

fn dot_then_keyword(n: &{point.if} Nested) {
    //~^ ERROR expected field name or tuple index after '.' in view
}

fn dot_then_symbol(n: &{point.+} Nested) {
    //~^ ERROR expected field name or tuple index after '.' in view
}

fn dot_then_string(n: &{point."x"} Nested) {
    //~^ ERROR expected field name or tuple index after '.' in view
}

fn dot_at_end(n: &{point.} Nested) {
    //~^ ERROR expected field name or tuple index after '.' in view
}

fn main() {}
