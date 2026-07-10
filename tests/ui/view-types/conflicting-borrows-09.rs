//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_09(a: &{0, 1} Pair, b: &mut {mut 0} Pair) -> i32 {
    b.0 += 1;
    a.0 + a.1 + b.0
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_09(&p, &mut p);
    //~^ ERROR cannot borrow
}
