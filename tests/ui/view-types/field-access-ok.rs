//@ check-pass

struct Data {
    x: i32,
    y: i32,
}

// Direct access through the parameter (no reborrow, no explicit local type).
fn access_param_no_annotation(rx: &{x} Data) -> i32 {
    rx.x
}

// Direct access through an explicitly-annotated local rebind.
fn access_annotated_local(rx: &{x} Data) -> i32 {
    let r: &{x} Data = rx;
    r.x
}

// Direct access through an unannotated local rebind (type inferred as &{x} Data).
fn access_inferred_local(rx: &{x} Data) -> i32 {
    let r = rx;
    r.x
}

fn main() {}
