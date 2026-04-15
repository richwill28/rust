//@ check-pass

struct Pair(i32, i32);

fn non_conflicting_borrows_09(a: &{0, 1} Pair, b: &{0} Pair) -> i32 {
    a.0 + a.1 + b.0
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = non_conflicting_borrows_09(&p, &p);
}
