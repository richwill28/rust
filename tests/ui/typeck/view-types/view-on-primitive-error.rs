// Test that views cannot be applied to primitive types

fn view_on_i32(x: &{field} i32) {
    //~^ ERROR only reference types to structs can be enriched with a view
}

fn view_on_str(s: &{field} str) {
    //~^ ERROR only reference types to structs can be enriched with a view
}

fn view_on_slice(s: &{field} [i32]) {
    //~^ ERROR only reference types to structs can be enriched with a view
}

fn main() {}
