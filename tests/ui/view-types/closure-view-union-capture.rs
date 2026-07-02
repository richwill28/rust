//@ check-pass
//@ edition:2021
// Tests that when a closure calls two methods with *different* view types on
// the same variable, the inferred capture view is the *union* of the two
// individual views rather than a conservative fallback to a full borrow.
//
// With union inference:
//   closure captures `self` as `&mut {counter, data} Self`
//
// This leaves `self.name` free to be borrowed outside the closure.

struct Storage {
    counter: usize,
    data: Vec<i32>,
    name: String,
}

impl Storage {
    /// View: only mutably borrows `counter`.
    fn increment_counter(&mut {mut counter} self) {
        self.counter += 1;
    }

    /// View: only mutably borrows `data`.
    fn push_data(&mut {mut data} self, v: i32) {
        self.data.push(v);
    }

    fn run(&mut self) {
        // Borrow a third field (`name`) for the duration of the block.
        // The closure touches `counter` (view {counter}) AND `data` (view {data}).
        // The merged capture view is {counter, data}, so `name` stays free.
        let name_ref: &String = &self.name;

        let mut update = || {
            self.increment_counter();
            self.push_data(42);
        };

        update();
        update();

        let _ = name_ref;
    }
}

fn main() {
    let mut s = Storage {
        counter: 0,
        data: vec![],
        name: String::from("hello"),
    };
    s.run();
    assert_eq!(s.counter, 2);
    assert_eq!(s.data, [42, 42]);
}
