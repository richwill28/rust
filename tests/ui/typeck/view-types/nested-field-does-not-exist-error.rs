// Test error when nested field path references non-existent field

struct Outer {
    inner: Inner,
}

struct Inner {
    value: i32,
}

fn nested_field_wrong(o: &{inner.nonexistent} Outer) {
    //~^ ERROR field `nonexistent` does not exist on struct
}

fn first_level_wrong(o: &{wrong.value} Outer) {
    //~^ ERROR field `wrong` does not exist on struct
}

fn main() {}
