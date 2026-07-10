//@ check-pass

struct S {
    next_id: usize,
    data: Vec<i32>,
}

impl S {
    fn new_id(&mut {mut next_id} self) -> usize {
        let i = self.next_id;
        self.next_id += 1;
        i
    }

    fn assign_id(&mut self, idx: usize) {
        if let Some(item) = self.data.get_mut(idx) {
            let id = self.new_id();  // Should NOT error.
            *item = id as i32;
        }
    }
}

fn main() {
    let mut s = S { next_id: 0, data: vec![0, 0, 0] };
    s.assign_id(0);
    s.assign_id(1);
    s.assign_id(2);
    assert_eq!(s.data, vec![0, 1, 2]);
}
