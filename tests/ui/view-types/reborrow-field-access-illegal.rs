//@ check-fail

struct Data {
    x: i32,
    y: i32,
}

// After reborrow, accessing a field that is not in the view is rejected.
fn reborrow_field_access_illegal(rx: &{x} Data) {
    let r = &*rx; // r: &{x} Data (view inherited)
    let _ = r.y; //~ERROR field `y` is not accessible through this view
}

fn main() {}
