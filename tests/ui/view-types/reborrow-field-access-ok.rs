//@ check-pass

struct Data {
    x: i32,
    y: i32,
}

// After reborrow, accessing a field that is in the view is allowed.
fn reborrow_field_access_ok(rx: &{x} Data) {
    let r = &*rx; // r: &{x} Data (view inherited)
    let _ = r.x; // OK
}

fn main() {}
