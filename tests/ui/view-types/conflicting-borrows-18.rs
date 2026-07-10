//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_18(a: &Pair, b: &mut {0, mut 1} Pair) -> i32 {
    b.1 += 1;
    a.0 + a.1 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_18(&p, &mut p);
    //~^ ERROR cannot borrow
}
