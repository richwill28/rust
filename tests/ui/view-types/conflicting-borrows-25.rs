//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_25(a: &mut {mut 1} Pair, b: &mut {0, mut 1} Pair) -> i32 {
    a.1 += 1;
    b.1 += 1;
    a.1 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_25(&mut p, &mut p);
    //~^ ERROR cannot borrow
}
