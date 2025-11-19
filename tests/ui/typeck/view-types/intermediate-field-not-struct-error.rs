// Test error when intermediate field in path is not a struct

struct Container {
    value: i32,
    nested: Nested,
}

struct Nested {
    data: i32,
}

fn intermediate_not_struct(c: &{value.x} Container) {
    //~^ ERROR cannot access `x` on `value` because it is not a struct
}

fn valid_nested(c: &{nested.data} Container) {
    // This should be fine
}

fn main() {}
