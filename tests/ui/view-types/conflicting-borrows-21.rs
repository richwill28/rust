//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_21(a: &mut {mut 0} Pair, b: &mut {mut 0, 1} Pair) -> i32 {
    a.0 += 1;
    b.0 += 1;
    a.0 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_21(&mut p, &mut p);
    //~^ ERROR cannot borrow
}
