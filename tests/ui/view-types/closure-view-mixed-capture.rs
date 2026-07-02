//@ check-pass
//@ edition:2021
// Regression test for the "view union in compute_min_captures" fix.
//
// When a closure body mixes:
//   (a) view-typed method calls   -> `borrow_with_view(*self, view)`
//   (b) direct field accesses     -> `borrow(*self/field, Mutable, None)`
//
// compute_min_captures must merge (b) into the ancestor capture (a) by
// extending the ancestor's view with the accessed field path, NOT by
// dropping the view restriction entirely to a full borrow.
//
// Without the fix, the ancestor's view was silently wiped out when any
// descendant with a `None` view was absorbed, making the closure appear to
// borrow all of `*self` and conflicting with simultaneous external borrows.

struct Header {
    data_page_info: Vec<u64>,
}

struct Storage {
    next_free_page: u64,
    header_fields: Header,
    header_dirty: bool,
    data_chunks: Vec<Option<u64>>,
}

impl Storage {
    /// View type: only touches `next_free_page`.
    fn assign_next_page(&mut {mut next_free_page} self) -> u64 {
        let p = self.next_free_page;
        self.next_free_page += 1;
        p
    }

    fn prepare(&mut self, kind: usize) {
        // Borrow `data_chunks[kind]` externally.
        let slot = &mut self.data_chunks[kind];

        // The closure calls `assign_next_page` (view {next_free_page}) AND
        // directly mutates `header_fields.data_page_info` and `header_dirty`.
        // After the fix, compute_min_captures builds up the view
        //   {next_free_page, header_fields.data_page_info, header_dirty}
        // which is disjoint from `data_chunks`, so the borrow above is fine.
        let mut init = || {
            let page = self.assign_next_page();
            if self.header_fields.data_page_info.len() <= kind {
                self.header_fields.data_page_info.resize(kind + 1, 0);
            }
            self.header_fields.data_page_info[kind] = page;
            self.header_dirty = true;
            page
        };

        *slot = Some(init());
    }
}

fn main() {
    let mut s = Storage {
        next_free_page: 1,
        header_fields: Header { data_page_info: vec![] },
        header_dirty: false,
        data_chunks: vec![None, None],
    };
    s.prepare(0);
    assert_eq!(s.next_free_page, 2);
    assert_eq!(s.header_fields.data_page_info[0], 1);
    assert!(s.header_dirty);
    assert_eq!(s.data_chunks[0], Some(1));
}
