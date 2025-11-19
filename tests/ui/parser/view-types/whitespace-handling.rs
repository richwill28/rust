// Test that view syntax handles various whitespace patterns
//@ check-pass

struct Point {
    x: i32,
    y: i32,
    z: i32,
}

// No spaces
fn no_spaces(p: &{x,y,z} Point) {}

// Extra spaces
fn extra_spaces(p: &{  x  ,  y  ,  z  } Point) {}

// Newlines in view
fn with_newlines(p: &{
    x,
    y,
    z
} Point) {}

// Mixed whitespace
fn mixed_whitespace(p: &{x,
    y, z} Point) {}

// Tabs
fn with_tabs(p: &{	x,	y,	z} Point) {}

fn main() {}
