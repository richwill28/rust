//@ check-fail

struct Data {
    x: i32,
    y: i32,
}

// Direct access through the parameter (no reborrow, no explicit local type).
fn access_param_no_annotation(rx: &{x} Data) {
    let _ = rx.y; //~ERROR field `y` is not accessible through this view
}

// Direct access through an explicitly-annotated local rebind.
fn access_annotated_local(rx: &{x} Data) {
    let r: &{x} Data = rx;
    let _ = r.y; //~ERROR field `y` is not accessible through this view
}

// Direct access through an unannotated local rebind (type inferred as &{x} Data).
fn access_inferred_local(rx: &{x} Data) {
    let r = rx;
    let _ = r.y; //~ERROR field `y` is not accessible through this view
}

fn main() {}
