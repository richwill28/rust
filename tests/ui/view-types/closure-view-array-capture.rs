//@ check-pass
//@ edition:2021
// Regression test: a closure's view-restricted capture of `*self` must be recognised
// as disjoint from a simultaneously live borrow of an inline array element field.
//
// This exercises three fixes working together:
//
// 1. `compute_min_captures` (upvar.rs): when a view-restricted ancestor capture
//    absorbs direct field-access descendants, `try_promote_descendant_to_view`
//    builds the union view `{next_free_page, header_dirty, header_fields.data_page_info}`
//    rather than falling back to a full borrow of `*self`.
//
// 2. `view_borrow_conflicts_with_access` (places_conflict.rs): when the access path
//    contains a non-Field projection (e.g., `Index`) after a field that diverges from
//    the view, we must not conservatively report a conflict. Previously, seeing
//    `data_chunks[kind]` caused an immediate `return true` before checking that
//    `data_chunks` was not in the view.
//
// 3. `determine_capture_info` (upvar.rs): two view-restricted captures of the same
//    place are unioned (`{next_free_page} ∪ {header_dirty}` etc.) rather than
//    dropped to a full borrow.
//
// The key stress: `data_chunks` is an INLINE ARRAY `[Option<u64>; N]`. Its element
// borrow persists through the `data_chunks` field for the duration of the call,
// so the view-disjointness check in `places_conflict` is exercised at conflict time.

struct Header {
    data_page_info: Vec<u64>,
}

const NUM_CHUNKS: usize = 4;

struct Storage {
    // Inline array — element borrow persists through the `data_chunks` field.
    data_chunks: [Option<u64>; NUM_CHUNKS],
    // Other fields accessed inside the closure:
    next_free_page: u64,
    header_fields: Header,
    header_dirty: bool,
}

impl Storage {
    /// View type: only mutably accesses `next_free_page`.
    fn assign_next_page(&mut {mut next_free_page} self) -> u64 {
        let page = self.next_free_page;
        self.next_free_page += 1;
        page
    }

    /// Lazy-initialise a slot: borrows `data_chunks[kind]` via `get_or_insert_with`
    /// while the closure mixes a view-typed call with direct field mutations.
    /// The merged closure capture `&mut {next_free_page, header_fields.data_page_info,
    /// header_dirty} Self` must be recognised as disjoint from `data_chunks`.
    fn prepare(&mut self, kind: usize) -> u64 {
        let state = self.data_chunks[kind].get_or_insert_with(|| {
            // view call: captures *self with view {next_free_page}
            let page = self.assign_next_page();

            // direct field access: promotes to view field {header_fields.data_page_info}
            if self.header_fields.data_page_info.len() <= kind {
                self.header_fields.data_page_info.resize(kind + 1, 0);
            }
            self.header_fields.data_page_info[kind] = page;

            // direct mutation: promotes to view field {header_dirty}
            self.header_dirty = true;

            page
        });
        *state
    }
}

fn main() {
    let mut s = Storage {
        data_chunks: [None; NUM_CHUNKS],
        next_free_page: 1,
        header_fields: Header { data_page_info: vec![] },
        header_dirty: false,
    };

    // First call: initialises slot 0.
    let v = s.prepare(0);
    assert_eq!(v, 1);
    assert_eq!(s.next_free_page, 2);
    assert!(s.header_dirty);
    assert_eq!(s.header_fields.data_page_info[0], 1);
    assert_eq!(s.data_chunks[0], Some(1));

    // Second call: slot already filled, closure never runs.
    let v2 = s.prepare(0);
    assert_eq!(v2, 1);
    assert_eq!(s.next_free_page, 2); // unchanged
}
