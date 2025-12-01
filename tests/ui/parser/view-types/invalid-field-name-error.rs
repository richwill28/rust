// Test invalid tokens as field names in views

struct Point {
    x: i32,
    y: i32,
}

fn invalid_string(p: &{"field"} Point) {
    //~^ ERROR expected field name or tuple index in view
}

fn invalid_keyword(p: &{if} Point) {
    //~^ ERROR expected field name or tuple index in view
}

fn invalid_symbol(p: &{+} Point) {
    //~^ ERROR expected field name or tuple index in view
}

fn main() {}
