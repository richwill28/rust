//@ check-pass

struct Data {
    x: i32,
    y: i32,
}

// When the requested view is narrower-or-equal to the source view,
// `&*rxy` produces the narrower view.
fn reborrow_view_narrow(rxy: &{x, y} Data) -> i32 {
    let r: &{x} Data = &*rxy; // {x} is narrower than {x, y} -> OK.
    r.x
}

fn main() {}
