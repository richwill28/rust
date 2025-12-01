// Test that disjoint field paths are accepted
//@ check-pass

struct Container {
    first: First,
    second: Second,
}

struct First {
    a: i32,
    b: i32,
}

struct Second {
    x: i32,
    y: i32,
}

fn disjoint_nested(c: &{first.a, second.x} Container) -> (&i32, &i32) {
    (&c.first.a, &c.second.x)
}

fn disjoint_same_level(c: &{first.a, first.b} Container) -> (&i32, &i32) {
    (&c.first.a, &c.first.b)
}

fn main() {}
