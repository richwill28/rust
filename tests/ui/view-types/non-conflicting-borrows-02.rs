//@ check-pass

struct Pair(i32, i32);

fn non_conflicting_borrows_02(a: &{0} Pair, b: &{1} Pair) -> i32 {
    a.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = non_conflicting_borrows_02(&p, &p);
}
