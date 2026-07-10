//@ check-fail

struct Pair(i32, i32);

fn conflicting_borrows_08(a: &{1} Pair, b: &mut Pair) -> i32 {
    b.0 += 1;
    b.1 += 1;
    a.1 + b.0 + b.1
}

fn main() {
    let mut p = Pair(0, 0);
    let _ = conflicting_borrows_08(&p, &mut p);
    //~^ ERROR cannot borrow
}
