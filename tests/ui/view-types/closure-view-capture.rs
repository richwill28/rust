//@ check-pass
//@ edition:2021
// View-aware closure capture requires Rust 2021 precise capture semantics.
// In earlier editions, `enable_precise_capture` is false and the compiler
// replaces inferred captures with coarse whole-variable captures, discarding
// the view restriction we record via `borrow_with_view`.

// Test that closures calling view-typed methods capture only the viewed fields,
// allowing disjoint borrows of other fields to coexist.

struct Storage {
    counter: usize,
    data: Vec<i32>,
}

impl Storage {
    // Only mutably accesses `counter`.
    fn increment_counter(&mut {mut counter} self) {
        self.counter += 1;
    }

    // Only reads `counter`.
    fn read_counter(&{counter} self) -> usize {
        self.counter
    }

    // Test: closure capturing `self` for a view-typed method call, while
    // `self.data` is simultaneously borrowed.
    fn process(&mut self) {
        let slice = &mut self.data;
        // The closure only borrows `self.counter` (via view type), so the
        // simultaneous borrow of `self.data` above should be fine.
        let mut bump = || self.increment_counter();
        bump();
        let _ = slice;
    }
}

fn main() {
    let mut s = Storage { counter: 0, data: vec![1, 2, 3] };
    s.process();
    assert_eq!(s.counter, 1);
}
