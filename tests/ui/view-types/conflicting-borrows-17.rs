//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_17(a: &Pair, b: &mut {mut 0, 1} Pair) -> i32 {
    b.0 += 1;
    a.0 + a.1 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_17(&p, &mut p);
    //~^ ERROR cannot borrow
}
