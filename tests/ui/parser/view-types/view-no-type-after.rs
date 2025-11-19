// Test error when view is not followed by a type

fn missing_type(p: &{x}) {
    //~^ ERROR expected type, found `)`
}

fn main() {}
