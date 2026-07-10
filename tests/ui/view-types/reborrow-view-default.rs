//@ check-pass

struct Data {
    x: i32,
    y: i32,
}

// When no view is explicitly requested, `&*rx` inherits the view from `rx`.
fn reborrow_view_default(rx: &{x} Data) -> i32 {
    let r = &*rx; // No type annotation on `r`.
    r.x
}

fn main() {}
