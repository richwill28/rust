//@ check-pass

struct Pair(i32, i32);

fn non_conflicting_borrows_04(a: &{0} Pair, b: &Pair) -> i32 {
    a.0 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = non_conflicting_borrows_04(&p, &p);
}
