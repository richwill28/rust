// Test that views work correctly with generic structs
//@ check-pass

struct Container<T> {
    value: T,
    count: i32,
}

fn view_on_generic_value<T>(c: &{value} Container<T>) -> &T {
    &c.value
}

fn view_on_generic_count<T>(c: &{count} Container<T>) -> &i32 {
    &c.count
}

fn view_on_generic_both<T>(c: &{value, count} Container<T>) -> (&T, &i32) {
    (&c.value, &c.count)
}

fn main() {}
