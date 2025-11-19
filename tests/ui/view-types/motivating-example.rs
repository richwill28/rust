//@ check-pass

// TODO: This test is marked as check-pass, but it should actually fail borrowck
// until view types are fully integrated with the borrow checker. Once borrow
// checker support is implemented, this should compile and demonstrate disjoint
// field borrowing through view types.

// This is the motivating example from the "View Types in Rust" paper (SPLASH 2025).
// It demonstrates the core problem that view types solve: allowing a method to
// specify which fields it accesses, enabling disjoint field borrowing.

struct S {
    next_id: usize,
    data: Vec<i32>,
}

impl S {
    // View type annotation: this method only mutably accesses `next_id`.
    fn new_id(&mut {mut next_id} self) -> usize {
        let i = self.next_id;
        self.next_id += 1;
        i
    }

    // This should compile because:
    // - The loop borrows `self.data` mutably.
    // - `self.new_id()` only borrows `self.next_id` mutably (per view type).
    // - `data` and `next_id` are disjoint fields.
    // - Therefore no conflict.
    fn assign_ids(&mut self) {
        for item in &mut self.data {
            let id = self.new_id();  // Should NOT error.
            *item = id as i32;
        }
    }
}

fn main() {
    let mut s = S { next_id: 0, data: vec![0, 0, 0] };
    s.assign_ids();
    assert_eq!(s.data, vec![0, 1, 2]);
}
