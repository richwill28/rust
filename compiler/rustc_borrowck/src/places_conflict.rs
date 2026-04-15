//! The borrowck rules for proving disjointness are applied from the "root" of the
//! borrow forwards, iterating over "similar" projections in lockstep until
//! we can prove overlap one way or another. Essentially, we treat `Overlap` as
//! a monoid and report a conflict if the product ends up not being `Disjoint`.
//!
//! On each step, if we didn't run out of borrow or place, we know that our elements
//! have the same type, and that they only overlap if they are the identical.
//!
//! For example, if we are comparing these:
//! ```text
//! BORROW:  (*x1[2].y).z.a
//! ACCESS:  (*x1[i].y).w.b
//! ```
//!
//! Then our steps are:
//! ```text
//!       x1         |   x1          -- places are the same
//!       x1[2]      |   x1[i]       -- equal or disjoint (disjoint if indexes differ)
//!       x1[2].y    |   x1[i].y     -- equal or disjoint
//!      *x1[2].y    |  *x1[i].y     -- equal or disjoint
//!     (*x1[2].y).z | (*x1[i].y).w  -- we are disjoint and don't need to check more!
//! ```
//!
//! Because `zip` does potentially bad things to the iterator inside, this loop
//! also handles the case where the access might be a *prefix* of the borrow, e.g.
//!
//! ```text
//! BORROW:  (*x1[2].y).z.a
//! ACCESS:  x1[i].y
//! ```
//!
//! Then our steps are:
//! ```text
//!       x1         |   x1          -- places are the same
//!       x1[2]      |   x1[i]       -- equal or disjoint (disjoint if indexes differ)
//!       x1[2].y    |   x1[i].y     -- equal or disjoint
//! ```
//!
//! -- here we run out of access - the borrow can access a part of it. If this
//! is a full deep access, then we *know* the borrow conflicts with it. However,
//! if the access is shallow, then we can proceed:
//!
//! ```text
//!       x1[2].y    | (*x1[i].y)    -- a deref! the access can't get past this, so we
//!                                     are disjoint
//! ```
//!
//! Our invariant is, that at each step of the iteration:
//!  - If we didn't run out of access to match, our borrow and access are comparable
//!    and either equal or disjoint.
//!  - If we did run out of access, the borrow can access a part of it.

use std::cmp::max;
use std::iter;

use rustc_hir as hir;
use rustc_middle::bug;
use rustc_middle::mir::{
    self, Body, BorrowKind, FakeBorrowKind, MutBorrowKind, Place, PlaceElem, PlaceRef,
    ProjectionElem,
};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Symbol;
use tracing::{debug, instrument};

use crate::{AccessDepth, ArtificialField, Deep, Overlap, Shallow};

/// When checking if a place conflicts with another place, this enum is used to influence decisions
/// where a place might be equal or disjoint with another place, such as if `a[i] == a[j]`.
/// `PlaceConflictBias::Overlap` would bias toward assuming that `i` might equal `j` and that these
/// places overlap. `PlaceConflictBias::NoOverlap` assumes that for the purposes of the predicate
/// being run in the calling context, the conservative choice is to assume the compared indices
/// are disjoint (and therefore, do not overlap).
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PlaceConflictBias {
    Overlap,
    NoOverlap,
}

/// Helper function for checking if places conflict with a mutable borrow and deep access depth.
/// This is used to check for places conflicting outside of the borrow checking code (such as in
/// dataflow).
///
/// Note: this is part of the public consumer API (re-exported via `consumers.rs` for use by
/// Clippy, Miri, and other external analysis tools). It is view-unaware and will report
/// conflicts even for fields not covered by a view borrow.
///
/// TODO(view): Make external consumers view-aware.
pub fn places_conflict<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    borrow_place: Place<'tcx>,
    access_place: Place<'tcx>,
    bias: PlaceConflictBias,
) -> bool {
    borrow_conflicts_with_place(
        tcx,
        body,
        borrow_place,
        BorrowKind::Mut { kind: MutBorrowKind::TwoPhaseBorrow },
        None,
        access_place.as_ref(),
        AccessDepth::Deep,
        bias,
    )
}

/// Like `places_conflict`, but accepts an optional view to restrict which fields of the
/// borrow place are considered borrowed.
pub(crate) fn places_conflict_with_view<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    borrow_place: Place<'tcx>,
    borrow_view: Option<mir::View<'tcx>>,
    access_place: Place<'tcx>,
    bias: PlaceConflictBias,
) -> bool {
    borrow_conflicts_with_place(
        tcx,
        body,
        borrow_place,
        BorrowKind::Mut { kind: MutBorrowKind::TwoPhaseBorrow },
        borrow_view,
        access_place.as_ref(),
        AccessDepth::Deep,
        bias,
    )
}

/// Checks whether the `borrow_place` conflicts with the `access_place` given a borrow kind and
/// access depth. The `bias` parameter is used to determine how the unknowable (comparing runtime
/// array indices, for example) should be interpreted - this depends on what the caller wants in
/// order to make the conservative choice and preserve soundness.
///
/// When `borrow_view` is `Some`, the borrow is restricted to only the fields named in the view.
/// An access to a field not in the view does not conflict with this borrow.
#[inline]
pub(super) fn borrow_conflicts_with_place<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    borrow_place: Place<'tcx>,
    borrow_kind: BorrowKind,
    borrow_view: Option<mir::View<'tcx>>,
    access_place: PlaceRef<'tcx>,
    access: AccessDepth,
    bias: PlaceConflictBias,
) -> bool {
    let borrow_local = borrow_place.local;
    let access_local = access_place.local;

    if borrow_local != access_local {
        // We have proven the borrow disjoint - further projections will remain disjoint.
        return false;
    }

    // This Local/Local case is handled by the more general code below, but
    // it's so common that it's a speed win to check for it first.
    if borrow_place.projection.is_empty() && access_place.projection.is_empty() {
        // Both refer to the same local with no projections. This is always a conflict,
        // regardless of view, accessing the entire local conflicts with any borrow of it.
        return true;
    }

    let base_conflict = place_components_conflict(
        tcx, body, borrow_place, borrow_kind, access_place, access, bias,
    );

    if !base_conflict {
        return false;
    }

    // If we have a view, check whether the access actually touches a field covered by the view.
    if let Some(view) = borrow_view {
        return view_borrow_conflicts_with_access(
            tcx, body, borrow_place, view, access_place,
        );
    }

    true
}

#[instrument(level = "debug", skip(tcx, body))]
fn place_components_conflict<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    borrow_place: Place<'tcx>,
    borrow_kind: BorrowKind,
    access_place: PlaceRef<'tcx>,
    access: AccessDepth,
    bias: PlaceConflictBias,
) -> bool {
    let borrow_local = borrow_place.local;
    let access_local = access_place.local;
    // borrow_conflicts_with_place should have checked that.
    assert_eq!(borrow_local, access_local);

    // loop invariant: borrow_c is always either equal to access_c or disjoint from it.
    for ((borrow_place, borrow_c), &access_c) in
        iter::zip(borrow_place.iter_projections(), access_place.projection)
    {
        debug!(?borrow_c, ?access_c);

        // Borrow and access path both have more components.
        //
        // Examples:
        //
        // - borrow of `a.(...)`, access to `a.(...)`
        // - borrow of `a.(...)`, access to `b.(...)`
        //
        // Here we only see the components we have checked so
        // far (in our examples, just the first component). We
        // check whether the components being borrowed vs
        // accessed are disjoint (as in the second example,
        // but not the first).
        match place_projection_conflict(tcx, body, borrow_place, borrow_c, access_c, bias) {
            Overlap::Arbitrary => {
                // We have encountered different fields of potentially
                // the same union - the borrow now partially overlaps.
                //
                // There is no *easy* way of comparing the fields
                // further on, because they might have different types
                // (e.g., borrows of `u.a.0` and `u.b.y` where `.0` and
                // `.y` come from different structs).
                //
                // We could try to do some things here - e.g., count
                // dereferences - but that's probably not a good
                // idea, at least for now, so just give up and
                // report a conflict. This is unsafe code anyway so
                // the user could always use raw pointers.
                debug!("arbitrary -> conflict");
                return true;
            }
            Overlap::EqualOrDisjoint => {
                // This is the recursive case - proceed to the next element.
            }
            Overlap::Disjoint => {
                // We have proven the borrow disjoint - further
                // projections will remain disjoint.
                debug!("disjoint");
                return false;
            }
        }
    }

    if borrow_place.projection.len() > access_place.projection.len() {
        for (base, elem) in borrow_place.iter_projections().skip(access_place.projection.len()) {
            // Borrow path is longer than the access path. Examples:
            //
            // - borrow of `a.b.c`, access to `a.b`
            //
            // Here, we know that the borrow can access a part of
            // our place. This is a conflict if that is a part our
            // access cares about.

            let base_ty = base.ty(body, tcx).ty;

            match (elem, base_ty.kind(), access) {
                (_, _, Shallow(Some(ArtificialField::ArrayLength)))
                | (_, _, Shallow(Some(ArtificialField::FakeBorrow))) => {
                    // The array length is like additional fields on the
                    // type; it does not overlap any existing data there.
                    // Furthermore, if cannot actually be a prefix of any
                    // borrowed place (at least in MIR as it is currently.)
                    //
                    // e.g., a (mutable) borrow of `a[5]` while we read the
                    // array length of `a`.
                    debug!("borrow_conflicts_with_place: implicit field");
                    return false;
                }

                (ProjectionElem::Deref, _, Shallow(None)) => {
                    // e.g., a borrow of `*x.y` while we shallowly access `x.y` or some
                    // prefix thereof - the shallow access can't touch anything behind
                    // the pointer.
                    debug!("borrow_conflicts_with_place: shallow access behind ptr");
                    return false;
                }
                (ProjectionElem::Deref, ty::Ref(_, _, hir::Mutability::Not, _), _) => {
                    // Shouldn't be tracked
                    bug!("Tracking borrow behind shared reference.");
                }
                (ProjectionElem::Deref, ty::Ref(_, _, hir::Mutability::Mut, _), AccessDepth::Drop) => {
                    // Values behind a mutable reference are not access either by dropping a
                    // value, or by StorageDead
                    debug!("borrow_conflicts_with_place: drop access behind ptr");
                    return false;
                }

                (ProjectionElem::Field { .. }, ty::Adt(def, _), AccessDepth::Drop) => {
                    // Drop can read/write arbitrary projections, so places
                    // conflict regardless of further projections.
                    if def.has_dtor(tcx) {
                        return true;
                    }
                }

                (ProjectionElem::Deref, _, Deep)
                | (ProjectionElem::Deref, _, AccessDepth::Drop)
                | (ProjectionElem::Field { .. }, _, _)
                | (ProjectionElem::Index { .. }, _, _)
                | (ProjectionElem::ConstantIndex { .. }, _, _)
                | (ProjectionElem::Subslice { .. }, _, _)
                | (ProjectionElem::OpaqueCast { .. }, _, _)
                | (ProjectionElem::Downcast { .. }, _, _)
                | (ProjectionElem::UnwrapUnsafeBinder(_), _, _) => {
                    // Recursive case. This can still be disjoint on a
                    // further iteration if this a shallow access and
                    // there's a deref later on, e.g., a borrow
                    // of `*x.y` while accessing `x`.
                }
            }
        }
    }

    // Borrow path ran out but access path may not
    // have. Examples:
    //
    // - borrow of `a.b`, access to `a.b.c`
    // - borrow of `a.b`, access to `a.b`
    //
    // In the first example, where we didn't run out of
    // access, the borrow can access all of our place, so we
    // have a conflict.
    //
    // If the second example, where we did, then we still know
    // that the borrow can access a *part* of our place that
    // our access cares about, so we still have a conflict.
    if borrow_kind == BorrowKind::Fake(FakeBorrowKind::Shallow)
        && borrow_place.projection.len() < access_place.projection.len()
    {
        debug!("borrow_conflicts_with_place: shallow borrow");
        false
    } else {
        debug!("borrow_conflicts_with_place: full borrow, CONFLICT");
        true
    }
}

// Given that the bases of `elem1` and `elem2` are always either equal
// or disjoint (and have the same type!), return the overlap situation
// between `elem1` and `elem2`.
fn place_projection_conflict<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    pi1: PlaceRef<'tcx>,
    pi1_elem: PlaceElem<'tcx>,
    pi2_elem: PlaceElem<'tcx>,
    bias: PlaceConflictBias,
) -> Overlap {
    match (pi1_elem, pi2_elem) {
        (ProjectionElem::Deref, ProjectionElem::Deref) => {
            // derefs (e.g., `*x` vs. `*x`) - recur.
            debug!("place_element_conflict: DISJOINT-OR-EQ-DEREF");
            Overlap::EqualOrDisjoint
        }
        (ProjectionElem::OpaqueCast(_), ProjectionElem::OpaqueCast(_)) => {
            // casts to other types may always conflict irrespective of the type being cast to.
            debug!("place_element_conflict: DISJOINT-OR-EQ-OPAQUE");
            Overlap::EqualOrDisjoint
        }
        (ProjectionElem::Field(f1, _), ProjectionElem::Field(f2, _)) => {
            if f1 == f2 {
                // same field (e.g., `a.y` vs. `a.y`) - recur.
                debug!("place_element_conflict: DISJOINT-OR-EQ-FIELD");
                Overlap::EqualOrDisjoint
            } else {
                let ty = pi1.ty(body, tcx).ty;
                if ty.is_union() {
                    // Different fields of a union, we are basically stuck.
                    debug!("place_element_conflict: STUCK-UNION");
                    Overlap::Arbitrary
                } else {
                    // Different fields of a struct (`a.x` vs. `a.y`). Disjoint!
                    debug!("place_element_conflict: DISJOINT-FIELD");
                    Overlap::Disjoint
                }
            }
        }
        (ProjectionElem::Downcast(_, v1), ProjectionElem::Downcast(_, v2)) => {
            // different variants are treated as having disjoint fields,
            // even if they occupy the same "space", because it's
            // impossible for 2 variants of the same enum to exist
            // (and therefore, to be borrowed) at the same time.
            //
            // Note that this is different from unions - we *do* allow
            // this code to compile:
            //
            // ```
            // fn foo(x: &mut Result<i32, i32>) {
            //     let mut v = None;
            //     if let Ok(ref mut a) = *x {
            //         v = Some(a);
            //     }
            //     // here, you would *think* that the
            //     // *entirety* of `x` would be borrowed,
            //     // but in fact only the `Ok` variant is,
            //     // so the `Err` variant is *entirely free*:
            //     if let Err(ref mut a) = *x {
            //         v = Some(a);
            //     }
            //     drop(v);
            // }
            // ```
            if v1 == v2 {
                debug!("place_element_conflict: DISJOINT-OR-EQ-FIELD");
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-FIELD");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::Index(..),
            ProjectionElem::Index(..)
            | ProjectionElem::ConstantIndex { .. }
            | ProjectionElem::Subslice { .. },
        )
        | (
            ProjectionElem::ConstantIndex { .. } | ProjectionElem::Subslice { .. },
            ProjectionElem::Index(..),
        ) => {
            // Array indexes (`a[0]` vs. `a[i]`). These can either be disjoint
            // (if the indexes differ) or equal (if they are the same).
            match bias {
                PlaceConflictBias::Overlap => {
                    // If we are biased towards overlapping, then this is the recursive
                    // case that gives "equal *or* disjoint" its meaning.
                    debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-INDEX");
                    Overlap::EqualOrDisjoint
                }
                PlaceConflictBias::NoOverlap => {
                    // If we are biased towards no overlapping, then this is disjoint.
                    debug!("place_element_conflict: DISJOINT-ARRAY-INDEX");
                    Overlap::Disjoint
                }
            }
        }
        (
            ProjectionElem::ConstantIndex { offset: o1, min_length: _, from_end: false },
            ProjectionElem::ConstantIndex { offset: o2, min_length: _, from_end: false },
        )
        | (
            ProjectionElem::ConstantIndex { offset: o1, min_length: _, from_end: true },
            ProjectionElem::ConstantIndex { offset: o2, min_length: _, from_end: true },
        ) => {
            if o1 == o2 {
                debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX");
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::ConstantIndex {
                offset: offset_from_begin,
                min_length: min_length1,
                from_end: false,
            },
            ProjectionElem::ConstantIndex {
                offset: offset_from_end,
                min_length: min_length2,
                from_end: true,
            },
        )
        | (
            ProjectionElem::ConstantIndex {
                offset: offset_from_end,
                min_length: min_length1,
                from_end: true,
            },
            ProjectionElem::ConstantIndex {
                offset: offset_from_begin,
                min_length: min_length2,
                from_end: false,
            },
        ) => {
            // both patterns matched so it must be at least the greater of the two
            let min_length = max(min_length1, min_length2);
            // `offset_from_end` can be in range `[1..min_length]`, 1 indicates the last
            // element (like -1 in Python) and `min_length` the first.
            // Therefore, `min_length - offset_from_end` gives the minimal possible
            // offset from the beginning
            if offset_from_begin >= min_length - offset_from_end {
                debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX-FE");
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX-FE");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false },
            ProjectionElem::Subslice { from, to, from_end: false },
        )
        | (
            ProjectionElem::Subslice { from, to, from_end: false },
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false },
        ) => {
            if (from..to).contains(&offset) {
                debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-CONSTANT-INDEX-SUBSLICE");
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-ARRAY-CONSTANT-INDEX-SUBSLICE");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false },
            ProjectionElem::Subslice { from, .. },
        )
        | (
            ProjectionElem::Subslice { from, .. },
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: false },
        ) => {
            if offset >= from {
                debug!("place_element_conflict: DISJOINT-OR-EQ-SLICE-CONSTANT-INDEX-SUBSLICE");
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-SLICE-CONSTANT-INDEX-SUBSLICE");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: true },
            ProjectionElem::Subslice { to, from_end: true, .. },
        )
        | (
            ProjectionElem::Subslice { to, from_end: true, .. },
            ProjectionElem::ConstantIndex { offset, min_length: _, from_end: true },
        ) => {
            if offset > to {
                debug!(
                    "place_element_conflict: \
                       DISJOINT-OR-EQ-SLICE-CONSTANT-INDEX-SUBSLICE-FE"
                );
                Overlap::EqualOrDisjoint
            } else {
                debug!("place_element_conflict: DISJOINT-SLICE-CONSTANT-INDEX-SUBSLICE-FE");
                Overlap::Disjoint
            }
        }
        (
            ProjectionElem::Subslice { from: f1, to: t1, from_end: false },
            ProjectionElem::Subslice { from: f2, to: t2, from_end: false },
        ) => {
            if f2 >= t1 || f1 >= t2 {
                debug!("place_element_conflict: DISJOINT-ARRAY-SUBSLICES");
                Overlap::Disjoint
            } else {
                debug!("place_element_conflict: DISJOINT-OR-EQ-ARRAY-SUBSLICES");
                Overlap::EqualOrDisjoint
            }
        }
        (ProjectionElem::Subslice { .. }, ProjectionElem::Subslice { .. }) => {
            debug!("place_element_conflict: DISJOINT-OR-EQ-SLICE-SUBSLICES");
            Overlap::EqualOrDisjoint
        }
        (
            ProjectionElem::Deref
            | ProjectionElem::Field(..)
            | ProjectionElem::Index(..)
            | ProjectionElem::ConstantIndex { .. }
            | ProjectionElem::OpaqueCast { .. }
            | ProjectionElem::Subslice { .. }
            | ProjectionElem::Downcast(..),
            _,
        ) => bug!(
            "mismatched projections in place_element_conflict: {:?} and {:?}",
            pi1_elem,
            pi2_elem
        ),

        (ProjectionElem::UnwrapUnsafeBinder(_), _) => {
            todo!()
        }
    }
}

/// Resolves a `FieldIdx` to its `Symbol` name given the type of the base place.
/// Returns `None` if the type is not a struct or the field index is out of bounds.
fn resolve_field_name<'tcx>(
    _tcx: TyCtxt<'tcx>,
    base_ty: Ty<'tcx>,
    field_idx: rustc_abi::FieldIdx,
) -> Option<Symbol> {
    match base_ty.kind() {
        ty::Adt(def, _) if def.is_struct() => {
            let variant = def.non_enum_variant();
            variant.fields.get(field_idx).map(|f| f.name)
        }
        // For tuples, closures, etc., view types don't apply
        _ => None,
    }
}

/// Determines whether an access place conflicts with a view-restricted borrow.
///
/// A view restricts a borrow to only cover specific fields. For example, a borrow of `s` with
/// view `{mut a, b}` only covers `s.a` and `s.b`. An access to `s.c` does NOT conflict.
///
/// This function is called after `place_components_conflict` has already determined that the
/// places would conflict without view information. The view can only *remove* a conflict: if
/// the access targets a field not covered by the view, the borrow does not actually conflict.
///
/// If the access covers the borrow's base (or is the same place), it always conflicts, you
/// can't move or overwrite `s` while any of its fields are borrowed. Otherwise, the access
/// extends past the borrow place into specific fields, and we check whether the accessed field
/// path overlaps any view field path using bidirectional prefix matching.
///
/// Examples:
///
/// ```rust,ignore
/// struct Inner { x: i32, y: i32 }
/// struct S { point: Inner, other: i32 }
/// let mut s = S { point: Inner { x: 1, y: 2 }, other: 3 };
/// let r = &{mut point.x} s;  // view borrows only s.point.x
///
/// s.point = ...;      // ERROR: [point] is a prefix of view path [point, x]
/// s.point.x = 10;     // ERROR: [point, x] exactly matches view path [point, x]
/// s.point.y = 20;     // OK: [point, y] does not match [point, x]
/// s.other = 30;       // OK: [other] does not match [point, x]
/// ```
fn view_borrow_conflicts_with_access<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &Body<'tcx>,
    borrow_place: Place<'tcx>,
    view: mir::View<'tcx>,
    access_place: PlaceRef<'tcx>,
) -> bool {
    let borrow_proj_len = borrow_place.projection.len();
    let access_proj_len = access_place.projection.len();

    // If the access place is shorter than or equal to the borrow place, the access covers
    // the borrow's base (or is the same). This always conflicts, accessing `s` when we
    // have a view borrow of `s.{a}` still conflicts because `s` includes `s.a`.
    if access_proj_len <= borrow_proj_len {
        debug!("view_borrow_conflicts: access is prefix/equal to borrow -> CONFLICT");
        return true;
    }

    // The access place extends past the borrow place. Collect the field projections
    // past the borrow place and check if they match any field path in the view.
    //
    // The borrow place projections have already been checked to match the access place
    // projections in lockstep (by place_components_conflict). So we only need to look at
    // the access projections starting from borrow_proj_len.
    let access_suffix = &access_place.projection[borrow_proj_len..];

    // Build the field path from the access suffix by resolving each Field projection.
    let mut access_field_path: Vec<Symbol> = Vec::new();
    let mut current_ty = if borrow_proj_len == 0 {
        body.local_decls[borrow_place.local].ty
    } else {
        Place::ty_from(
            borrow_place.local,
            &borrow_place.projection[..borrow_proj_len],
            body,
            tcx,
        )
        .ty
    };

    for &proj_elem in access_suffix {
        match proj_elem {
            ProjectionElem::Field(idx, ty) => {
                if let Some(name) = resolve_field_name(tcx, current_ty, idx) {
                    access_field_path.push(name);
                    current_ty = ty;
                } else {
                    // Can't resolve field name (tuple, closure, etc.), conservatively conflict.
                    debug!("view_borrow_conflicts: can't resolve field name -> CONFLICT");
                    return true;
                }
            }
            ProjectionElem::Deref => {
                // Going through a deref past the borrow place, the view doesn't restrict
                // what's behind a pointer. Conservatively report conflict.
                debug!("view_borrow_conflicts: deref past borrow place -> CONFLICT");
                return true;
            }
            _ => {
                // Index, ConstantIndex, Subslice, Downcast, etc., conservatively conflict.
                debug!("view_borrow_conflicts: non-field projection -> CONFLICT");
                return true;
            }
        }
    }

    if access_field_path.is_empty() {
        // No field projections beyond the borrow place, shouldn't happen since we checked
        // access_proj_len > borrow_proj_len, but be conservative.
        debug!("view_borrow_conflicts: no field path extracted -> CONFLICT");
        return true;
    }

    // Check if any view field's path is a prefix of (or equal to) the access field path,
    // or if the access field path is a prefix of any view field's path.
    //
    // This mirrors the prefix-matching semantics: a view field `{point}` covers
    // `point`, `point.x`, `point.x.y`, etc. And an access to `point` conflicts with
    // a view field `{point.x}` because it covers the parent.
    for view_field in view.iter() {
        let view_path = view_field.path;

        // Check if view_path is a prefix of access_field_path
        if access_field_path.len() >= view_path.len()
            && access_field_path[..view_path.len()] == *view_path
        {
            debug!(
                "view_borrow_conflicts: view field {:?} is prefix of access {:?} -> CONFLICT",
                view_path, access_field_path
            );
            return true;
        }

        // Check if access_field_path is a prefix of view_path
        if view_path.len() >= access_field_path.len()
            && view_path[..access_field_path.len()] == *access_field_path
        {
            debug!(
                "view_borrow_conflicts: access {:?} is prefix of view field {:?} -> CONFLICT",
                access_field_path, view_path
            );
            return true;
        }
    }

    // No view field matches the access path, the access is to a field not covered by the view.
    debug!(
        "view_borrow_conflicts: access {:?} not in view -> NO CONFLICT",
        access_field_path
    );
    false
}
