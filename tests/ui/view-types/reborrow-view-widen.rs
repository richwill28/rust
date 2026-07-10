//@ check-fail

struct Data {
    x: i32,
    y: i32,
}

// Attempting to widen a view reference via explicit reborrow should be rejected.
fn reborrow_view_widen(rx: &{x} Data) {
    let r: &Data = &*rx; //~ERROR mismatched types
}

fn main() {}
