// Test that views cannot be applied to enums

enum Color {
    Red,
    Green,
    Blue,
}

fn view_on_enum(c: &{Red} Color) {
    //~^ ERROR only reference types to structs can be enriched with a view
}

fn main() {}
