// Test error when view contains overlapping nested field paths

struct Outer {
    inner: Inner,
}

struct Inner {
    value: i32,
}

fn prefix_overlap(o: &{inner, inner.value} Outer) {
    //~^ ERROR view contains overlapping field paths
}

fn prefix_overlap_reverse(o: &{inner.value, inner} Outer) {
    //~^ ERROR view contains overlapping field paths
}

fn main() {}
