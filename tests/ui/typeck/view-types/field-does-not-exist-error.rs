// Test error when view references a non-existent field

struct Point {
    x: i32,
    y: i32,
}

fn nonexistent_field(p: &{z} Point) {
    //~^ ERROR field `z` does not exist on struct
}

fn multiple_fields_one_wrong(p: &{x, z} Point) {
    //~^ ERROR field `z` does not exist on struct
}

fn main() {}
