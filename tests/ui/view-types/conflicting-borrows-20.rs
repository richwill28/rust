//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_20(a: &mut {mut 0} Pair, b: &mut {mut 0} Pair) -> i32 {
    a.0 += 1;
    b.0 += 1;
    a.0 + b.0
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_20(&mut p, &mut p);
    //~^ ERROR cannot borrow
}
