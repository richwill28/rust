//! View types checking.
//!
//! This module implements the logic for checking view types during HIR type checking.
//! View types allow restricting which fields of a struct are accessible through a reference,
//! providing fine-grained control over data access patterns.

use rustc_errors::ErrorGuaranteed;
use rustc_hir as hir;
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, Ty};
use rustc_span::{Span, Symbol};
use rustc_span::edit_distance::find_best_match_for_name;

use crate::FnCtxt;

/// Information about an active view constraint.
#[derive(Debug, Clone)]
pub(crate) struct ViewConstraint<'tcx> {
    /// The HIR view definition.
    pub(crate) view: &'tcx hir::View<'tcx>,
    /// The struct DefId that this view constrains.
    pub(crate) struct_def_id: DefId,
    /// The span where this constraint was established.
    pub(crate) span: Span,
}

impl<'tcx> ViewConstraint<'tcx> {
    /// Creates a new view constraint.
    pub(crate) fn new(
        view: &'tcx hir::View<'tcx>,
        struct_def_id: DefId,
        span: Span,
    ) -> Self {
        ViewConstraint {
            view,
            struct_def_id,
            span,
        }
    }
}

// ============================================================================
// VIEW TYPE WELL-FORMEDNESS CHECKING
// ============================================================================

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    /// Checks well-formedness of view type definitions in HIR.
    ///
    /// View types restrict field access through references using syntax like `&{field, ..} T`.
    /// This function checks that such view type definitions are valid:
    ///
    /// - Views only appear on reference types (`&T`, `&mut T`).
    /// - The target type `T` must be a struct (not enum, union, or primitive).
    /// - All fields mentioned in the view actually exist on the target struct.
    /// - Fields in the view are disjoint (no field is a prefix of another).
    /// - If any field in the view has `mut`, the reference must be `&mut`.
    /// - Public functions cannot expose private fields through views.
    pub(crate) fn check_view_ty_wf(
        &self,
        hir_ty: &'tcx hir::Ty<'tcx>,
    ) -> Result<(), ErrorGuaranteed> {
        match &hir_ty.kind {
            hir::TyKind::Ref(_, mut_ty, Some(view)) => {
                if let Some(def_id) = self.extract_def_id_from_hir_type(&mut_ty.ty) {
                    // Does the target type support views at all?
                    let target_ty = self.tcx.type_of(def_id).instantiate_identity();
                    self.check_ty_supports_views(target_ty, hir_ty, def_id)?;
                    // Do all referenced fields actually exist?
                    self.check_view_fields_exist(view, def_id, hir_ty.span)?;
                    // Are all fields disjoint?
                    self.check_view_fields_disjoint(view, hir_ty.span)?;
                    // Is mutability consistent?
                    self.check_view_mutability_consistency(view, mut_ty.mutbl, hir_ty.span)?;
                    // Are we exposing private fields inappropriately?
                    self.check_view_visibility_constraints(view, def_id, hir_ty.span)?;
                } else {
                    // No DefId means this is not a named type (e.g., primitive, slice, etc.)
                    // Views are only allowed on struct types, so this is an error.
                    return Err(self.report_view_on_non_named_type_error(hir_ty));
                }
                Ok(())
            }
            _ => Ok(()), // Not a view type.
        }
    }

    /// Checks if a type supports views and reports an error if not.
    pub(crate) fn check_ty_supports_views(
        &self, 
        ty: Ty<'tcx>, 
        hir_ty: &'tcx hir::Ty<'tcx>, 
        def_id: DefId
    ) -> Result<(), ErrorGuaranteed> {
        if !matches!(ty.kind(), ty::Adt(adt_def, _) if adt_def.is_struct()) {
            return Err(self.report_invalid_view_target_error(hir_ty, def_id));
        }
        Ok(())
    }

    /// Checks that all fields referenced in a view actually exist in the target struct.
    /// This recursively checks nested field paths like `field.subfield.subsubfield`.
    pub(crate) fn check_view_fields_exist(
        &self,
        view: &hir::View<'tcx>,
        def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        for view_field in view.fields {
            self.check_view_field_path(view_field, def_id, view_span)?;
        }
        Ok(())
    }

    /// Checks that a view field path exists and is well-formed.
    /// 
    /// For a path like `field.subfield.subsubfield`, this:
    /// 1. Checks that `field` exists in the root struct.
    /// 2. Checks that `field` is itself a struct.
    /// 3. Recursively checks that `subfield` exists in `field`'s struct.
    /// 4. Continues until the entire path is checked.
    fn check_view_field_path(
        &self,
        view_field: &hir::ViewField<'tcx>,
        struct_def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        if view_field.path.is_empty() {
            return Err(self.report_invalid_view_field_path_error(view_field, view_span));
        }

        self.check_field_path_at_index(
            view_field,
            0, // Start at index 0.
            struct_def_id,
            view_span,
        )
    }

    /// Checks a field path starting at the given index.
    /// 
    /// This recursively walks through the path components, ensuring each field exists
    /// and that intermediate fields are structs that support nested access.
    fn check_field_path_at_index(
        &self,
        view_field: &hir::ViewField<'tcx>,
        path_index: usize,
        current_struct_def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        let path = &view_field.path;
    
        if path_index >= path.len() {
            // We've successfully checked the entire path.
            return Ok(());
        }

        let field_name = path[path_index];

        let struct_ty = self.tcx.type_of(current_struct_def_id).instantiate_identity();
        let ty::Adt(adt_def, _) = struct_ty.kind() else {
            // This shouldn't happen if we checked type support correctly.
            unreachable!("expected struct type after type support validation, got {:?}", struct_ty.kind());
        };

        let variant = adt_def.non_enum_variant();

        // Find the field in the current struct.
        let field_def = variant.fields.iter().find(|f| f.name == field_name);
        let Some(field_def) = field_def else {
            // Field doesn't exist in this struct.
            return Err(self.report_view_field_not_found_error(
                view_field,
                field_name,
                current_struct_def_id,
                view_span,
            ));
        };

        // Check if this is the last component in the path.
        if path_index == path.len() - 1 {
            // This is the final field, we're done.
            return Ok(());
        }

        // This is an intermediate field, it must be a struct for nested access.
        let field_ty = self.tcx.type_of(field_def.did).instantiate_identity();
        let field_struct_def_id = self.get_struct_def_id(field_ty);
        let Some(field_struct_def_id) = field_struct_def_id else {
            // Intermediate field is not a struct, can't have nested access.
            return Err(self.report_non_struct_field_in_view_path_error(
                view_field,
                field_ty,
                path_index,
                view_span,
            ));
        };

        // Recursively check the rest of the path.
        self.check_field_path_at_index(
            view_field,
            path_index + 1,
            field_struct_def_id,
            view_span,
        )
    }

    /// Checks that all fields in a view are disjoint.
    /// 
    /// Two fields are disjoint if neither is a prefix of the other. This prevents:
    /// - Duplicate fields: `{x, x}`.
    /// - Overlapping nested paths: `{point, point.x}` (point is a prefix of point.x).
    fn check_view_fields_disjoint(
        &self,
        view: &hir::View<'tcx>,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        // Check each pair of fields for disjointness
        for (i, field1) in view.fields.iter().enumerate() {
            for field2 in view.fields.iter().skip(i + 1) {
                // Check if either field is a prefix of the other
                if self.is_field_path_prefix(field1, field2) {
                    return Err(self.report_view_fields_not_disjoint_error(
                        field1,
                        field2,
                        view_span,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Checks mutability consistency between view fields and reference type.
    /// 
    /// If any field in the view has `mut`, the reference itself must be `&mut`.
    /// This ensures that mutable access is only possible through mutable references.
    fn check_view_mutability_consistency(
        &self,
        view: &hir::View<'tcx>,
        ref_mutability: hir::Mutability,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        let has_mutable_fields = view.fields.iter().any(|field| field.mutbl == hir::Mutability::Mut);
        if has_mutable_fields && ref_mutability == hir::Mutability::Not {
            return Err(self.report_view_mutability_mismatch_error(view, view_span));
        }
        Ok(())
    }

    /// Checks visibility constraints for view fields.
    /// 
    /// Public functions cannot expose private fields through views, as this would
    /// violate privacy boundaries and allow external code to access private data.
    fn check_view_visibility_constraints(
        &self,
        view: &hir::View<'tcx>,
        def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        // First, determine if we're in a public context.
        let in_public_context = self.is_in_public_function_context();

        if !in_public_context {
            // If we're not in a public context, visibility constraints don't apply.
            return Ok(());
        }

        // Check each field in the view for visibility violations.
        // We need to recursively check all components in the field path.
        for view_field in view.fields {
            self.check_view_field_path_visibility(view_field, def_id, view_span)?;
        }

        Ok(())
    }

    /// Checks visibility constraints for a view field path recursively.
    /// 
    /// For a path like `field.subfield.subsubfield`, this checks that all
    /// components (`field`, `subfield`, and `subsubfield`) are visible
    /// from the current context.
    fn check_view_field_path_visibility(
        &self,
        view_field: &hir::ViewField<'tcx>,
        struct_def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        if view_field.path.is_empty() {
            return Ok(());
        }

        self.check_field_path_visibility_at_index(
            view_field,
            0, // Start at index 0.
            struct_def_id,
            view_span,
        )
    }

    /// Checks field path visibility starting at the given index.
    /// 
    /// This recursively walks through the path components, ensuring each field
    /// is visible from the current context.
    fn check_field_path_visibility_at_index(
        &self,
        view_field: &hir::ViewField<'tcx>,
        path_index: usize,
        current_struct_def_id: DefId,
        view_span: Span,
    ) -> Result<(), ErrorGuaranteed> {
        let path = &view_field.path;
    
        if path_index >= path.len() {
            // We've successfully checked the entire path.
            return Ok(());
        }

        let field_name = path[path_index];

        // Get the struct definition to check field visibility.
        let struct_ty = self.tcx.type_of(current_struct_def_id).instantiate_identity();
        let ty::Adt(adt_def, _) = struct_ty.kind() else {
            // This shouldn't happen if we checked type support correctly.
            unreachable!("expected struct type after type support validation, got {:?}", struct_ty.kind());
        };

        let variant = adt_def.non_enum_variant();

        // Find the field in the current struct.
        let field_def = variant.fields.iter().find(|f| f.name == field_name);
        let Some(field_def) = field_def else {
            // Field doesn't exist, this should have been caught by field existence checking.
            unreachable!("field `{}` should exist after field existence validation", field_name);
        };

        // Check if the field is visible from the current context.
        if !self.is_field_visible_from_current_context(field_def) {
            return Err(self.report_view_privacy_violation_error(
                view_field,
                field_name,
                current_struct_def_id,
                view_span,
            ));
        }

        // Check if this is the last component in the path.
        if path_index == path.len() - 1 {
            // This is the final field, we're done.
            return Ok(());
        }

        // This is an intermediate field, recursively check the rest of the path.
        let field_ty = self.tcx.type_of(field_def.did).instantiate_identity();
        let field_struct_def_id = self.get_struct_def_id(field_ty);
        let Some(field_struct_def_id) = field_struct_def_id else {
            // Intermediate field is not a struct, this should have been caught by field existence checking.
            unreachable!("field `{}` should be a struct after field existence validation", field_name);
        };

        // Recursively check the rest of the path.
        self.check_field_path_visibility_at_index(
            view_field,
            path_index + 1,
            field_struct_def_id,
            view_span,
        )
    }
}

// ============================================================================
// EXPRESSION-LEVEL VIEW CHECKING
// ============================================================================

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    /// Checks if a field access satisfies the view.
    ///
    /// This function looks up the view constraint (if any) for the base expression
    /// and validates that the accessed field satisfies the view.
    pub(crate) fn check_field_access_satisfies_view(
        &self,
        field_expr: &'tcx hir::Expr<'tcx>,
    ) -> Result<(), ErrorGuaranteed> {
        // Build the full field access path from the expression tree.
        let mut access_path = Vec::new();
        let mut current_expr = field_expr;

        // Walk up the field access chain to build the complete path.
        // For example, `x.field.subfield` becomes [field, subfield].
        loop {
            match &current_expr.kind {
                hir::ExprKind::Field(base_expr, field) => {
                    access_path.insert(0, field.name);
                    current_expr = base_expr;
                }
                hir::ExprKind::Path(qpath) => {
                    // We've reached the variable reference at the base.
                    let res = self.typeck_results.borrow().qpath_res(qpath, current_expr.hir_id);

                    // Get the HirId of the binding. For local variables (parameters),
                    // the Res is Res::Local(hir_id) which directly gives us the binding's HirId.
                    let hir_id = match res {
                        hir::def::Res::Local(hir_id) => hir_id,
                        hir::def::Res::Def(_, def_id) => {
                            // For items (not local variables), convert DefId to HirId
                            let Some(local_def_id) = def_id.as_local() else {
                                return Ok(()); // Not a local item.
                            };
                            self.tcx.local_def_id_to_hir_id(local_def_id)
                        }
                        _ => return Ok(()), // Other cases don't have view constraints.
                    };

                    // Look up the view constraint for this variable.
                    let opt_view_constraint = self.view_constraints.borrow().get(&hir_id).cloned();
                    if let Some(ref view_constraint) = opt_view_constraint {
                        return self.check_access_path_against_view(
                            field_expr,
                            &access_path,
                            view_constraint,
                        );
                    }

                    // No annotation-based view constraint found. Fall back to checking the
                    // inferred type of the variable: a reborrowed view reference like
                    // `let r = &*rx` where `rx: &{x} Data` carries its view in the type
                    // `&{x} Data`, even though `r` has no explicit annotation.
                    let node_ty = self.typeck_results.borrow().node_type(current_expr.hir_id);
                    if let ty::Ref(_, _, _, Some(view)) = node_ty.kind() {
                        let field_allowed = view.iter().any(|vf| {
                            access_path.len() >= vf.path.len()
                                && access_path[..vf.path.len()] == *vf.path
                        });
                        if !field_allowed {
                            let access_str = access_path
                                .iter()
                                .map(|s| s.to_string())
                                .collect::<Vec<_>>()
                                .join(".");
                            let available: Vec<String> = view
                                .iter()
                                .map(|vf| {
                                    vf.path
                                        .iter()
                                        .map(|s| s.to_string())
                                        .collect::<Vec<_>>()
                                        .join(".")
                                })
                                .collect();
                            let mut err = self.dcx().struct_span_err(
                                field_expr.span,
                                format!("field `{}` is not accessible through this view", access_str),
                            );
                            err.span_label(field_expr.span, "field access not allowed by view");
                            if !available.is_empty() {
                                err.help(format!(
                                    "this view allows access to: {}",
                                    available.join(", ")
                                ));
                                err.note("you can access these fields and any of their subfields");
                            }
                            return Err(err.emit());
                        }
                    }
                    return Ok(());
                }
                _ => {
                    // Not a simple field access chain, no view constraint applies.
                    return Ok(());
                }
            }
        }
    }

    /// Checks that an access path is allowed by the view constraint.
    ///
    /// This only checks if the field is structurally allowed by the view (i.e., is it in
    /// the view at all). Mutation permission checking (whether the view field has `mut`)
    /// should be done in the borrow checker, following rustc's design where mutation
    /// permission is a borrow-checking concern, not a type-checking concern.
    ///
    /// TODO: Implement mutation permission checking in the borrow checker.
    /// When a field access through a view is used in a mutating context (assignment,
    /// `&mut` borrow, etc.), the borrow checker should verify that the corresponding
    /// view field has the `mut` qualifier. This is analogous to how rustc checks
    /// whether a variable is `mut` or a reference is `&mut` - those checks happen in
    /// borrowck, not in HIR typeck.
    fn check_access_path_against_view(
        &self,
        field_expr: &'tcx hir::Expr<'tcx>,
        access_path: &[Symbol],
        view_constraint: &ViewConstraint<'tcx>,
    ) -> Result<(), ErrorGuaranteed> {
        // Check if the access path is allowed by any view field.
        //
        // Access is allowed if the access path starts with (is a prefix extension of)
        // any view field path. This implements the semantics:
        // - View `{field}` allows `field`, `field.x`, `field.x.y`, etc.
        // - View `{field.x}` allows `field.x`, `field.x.y`, etc. (but NOT just `field`)
        let field_allowed = view_constraint.view.fields.iter().any(|view_field| {
            self.view_field_matches_access_path(view_field, access_path)
        });

        if field_allowed {
            Ok(())
        } else {
            // Field access violates the view - field not in view.
            Err(self.report_field_path_not_in_view_error(
                field_expr,
                access_path,
                view_constraint,
            ))
        }
    }

    /// Checks if a view field specification matches the field access path.
    ///
    /// Returns true if the access path starts with the view field path.
    /// For example:
    /// - View path [field] matches access [field], [field, x], [field, x, y]
    /// - View path [field, x] matches access [field, x], [field, x, y]
    /// - View path [field, x] does NOT match access [field] or [field, y]
    fn view_field_matches_access_path(
        &self,
        view_field: &hir::ViewField<'tcx>,
        access_path: &[Symbol],
    ) -> bool {
        // The access path must start with the view field path.
        access_path.len() >= view_field.path.len()
            && access_path[..view_field.path.len()] == *view_field.path
    }
}

// ============================================================================
// VIEW CONSTRAINT COLLECTION
// ============================================================================

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    /// Collects view constraint from a function parameter.
    ///
    /// This is called during function parameter type checking to extract and store
    /// any view constraints from the parameter's type annotation.
    pub(crate) fn collect_view_constraint_from_param(
        &self,
        param: &'tcx hir::Param<'tcx>,
        hir_ty: &'tcx hir::Ty<'tcx>,
    ) {
        if let Some(view_constraint) = self.extract_view_from_hir_type(hir_ty, param.span) {
            // Get the HirId from the param pattern.
            // For simple bindings (e.g. `fn foo(p: &{x} Point)`), this is straightforward.
            // We just extract the single binding's HirId and associate it with the view constraint.
            //
            // TODO: Complex bindings in function parameters are not yet supported.
            // Rust allows patterns in function parameters, such as:
            //   - Tuple patterns: `fn foo((a, b): (&{x} Point, &{y} Point))`
            //     Here `a` gets view `{x}` and `b` gets view `{y}`
            //   - Struct patterns through references: This requires all destructured fields to be in the view.
            //     Valid: `fn foo(&Point { x, y }: &&{x, y} Point)` - both x and y are in view
            //     Invalid: `fn foo(&Point { x, y }: &&{x} Point)` - y is not in view, can't destructure it
            //   - Reference patterns: `fn foo(&p: &&{x} Point)`
            //     Here `p` would be bound to a `&{x} Point`
            //   - Nested patterns: `fn foo(((a, b), n): ((&{x} Point, &{y} Point), i32))`
            //
            // For these complex patterns, we would need to:
            // 1. Walk the pattern tree recursively to find all `PatKind::Binding` nodes.
            // 2. Determine which view constraint applies to each binding (e.g. in tuple patterns,
            //    different tuple elements might have different view constraints).
            // 3. When destructuring structs with view constraints, validate that all accessed fields
            //    are allowed by the view constraint.
            //
            // Currently, if a complex pattern is used with a view type, the view constraint is
            // silently ignored since we only handle `PatKind::Binding` at the top level.
            if let hir::PatKind::Binding(_, hir_id, _, _) = param.pat.kind {
                self.view_constraints.borrow_mut().insert(hir_id, view_constraint);
            }
        }
    }

    /// Collects view constraint from a let binding.
    ///
    /// TODO: This is not yet implemented. Currently view types are only supported in function parameters.
    ///
    /// When implementing this, consider:
    /// - View subtyping/coercion: Should `let x: &{x} Point = y` be allowed when `y: &{x, y} Point`?
    /// - Reborrow semantics: How should `let x = &{x} *p` create a new borrow with restricted view?
    /// - Type inference: What happens when no type annotation is present?
    /// - Complex bindings: How to handle view constraints with destructuring patterns?
    #[allow(dead_code)]
    pub(crate) fn collect_view_constraint_from_local(
        &self,
        _local: &'tcx hir::LetStmt<'tcx>,
    ) {
        // Not yet implemented, see TODO above.
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    /// Extracts view information from an HIR type.
    fn extract_view_from_hir_type(
        &self,
        hir_ty: &'tcx hir::Ty<'tcx>,
        span: Span,
    ) -> Option<ViewConstraint<'tcx>> {
        match &hir_ty.kind {
            hir::TyKind::Ref(_, mut_ty, Some(view)) => {
                // Found a reference type with a view.
                let struct_def_id = self.extract_def_id_from_hir_type(&mut_ty.ty)?;
                Some(ViewConstraint::new(view, struct_def_id, span))
            }
            _ => None,
        }
    }

    /// Extracts the DefId from an HIR type (for structs that can have views).
    fn extract_def_id_from_hir_type(&self, hir_ty: &'tcx hir::Ty<'tcx>) -> Option<DefId> {
        match &hir_ty.kind {
            hir::TyKind::Path(qpath) => {
                // Resolve the path to get the DefId.
                let res = self.typeck_results.borrow().qpath_res(qpath, hir_ty.hir_id);
                if let Some(def_id) = res.opt_def_id() {
                    return Some(def_id);
                }
                // If we couldn't resolve directly (e.g., for implicit `self` receivers),
                // check if we're in an impl context and this is `Self`.
                if let hir::QPath::Resolved(_, path) = qpath {
                    if let hir::def::Res::SelfTyAlias { alias_to, .. } = path.res {
                        return Some(alias_to);
                    }
                    if let hir::def::Res::SelfTyParam { trait_: impl_def_id } = path.res {
                        // Get the self type from the impl
                        let impl_ty = self.tcx.type_of(impl_def_id).instantiate_identity();
                        return self.get_struct_def_id(impl_ty);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Gets the struct `DefId` from a type, if it's a struct.
    pub(crate) fn get_struct_def_id(&self, ty: Ty<'tcx>) -> Option<DefId> {
        match ty.kind() {
            ty::Adt(adt_def, _) if adt_def.is_struct() => Some(adt_def.did()),
            _ => None,
        }
    }

    /// Checks if one field path is a prefix of another (or vice versa).
    /// 
    /// Returns true if:
    /// - The paths are identical (duplicate).
    /// - One path is a prefix of the other (e.g. `point` and `point.x`).
    fn is_field_path_prefix(
        &self,
        field1: &hir::ViewField<'tcx>,
        field2: &hir::ViewField<'tcx>,
    ) -> bool {
        let path1 = &field1.path;
        let path2 = &field2.path;

        // Check if path1 is a prefix of path2
        if path1.len() <= path2.len() && path2.starts_with(path1) {
            return true;
        }

        // Check if path2 is a prefix of path1
        if path2.len() <= path1.len() && path1.starts_with(path2) {
            return true;
        }

        false
    }

    /// Determines if the current function context is public.
    /// 
    /// This is used to enforce visibility constraints: public functions
    /// cannot expose private fields through views.
    /// 
    /// We draw a clear boundary at full `pub` visibility: only fully public
    /// functions are restricted from exposing private fields. Functions with
    /// restricted visibility (pub(crate), pub(super), etc.) are treated as
    /// internal implementation details that can legitimately access private
    /// fields for their implementation.
    /// 
    /// This is sound because:
    /// - `pub(crate)` and `pub(super)` create encapsulation boundaries within
    ///   the crate, not at the crate boundary.
    /// - Code within the same crate or module is part of the implementation
    ///   and can coordinate using private fields.
    /// - Only fully `pub` items cross the crate boundary where privacy matters.
    /// - Views in restricted-visibility functions are an internal organizational
    ///   tool, not part of the public API contract.
    fn is_in_public_function_context(&self) -> bool {
        let fn_visibility = self.tcx.visibility(self.body_id.to_def_id());
        match fn_visibility {
            ty::Visibility::Public => true,
            ty::Visibility::Restricted(_) => {
                // Restricted visibility (pub(crate), pub(super), etc.) is treated
                // as non-public. These functions are internal to the crate/module
                // and can expose private fields through views without violating
                // encapsulation boundaries.
                false
            }
        }
    }

    /// Checks if a field is visible from the current context.
    /// 
    /// This considers the field's visibility and determines if it can be
    /// exposed in a public function context (where this check is called).
    /// 
    /// Since this function is only invoked when we're in a public function
    /// context (checked by `is_in_public_function_context`), we use a simple
    /// rule: only fully `pub` fields can be exposed.
    /// 
    /// This should be sound:
    /// - Fully public fields are always safe to expose in public functions.
    /// - Non-public fields (private, pub(crate), pub(super)) should not be
    ///   exposed through public function signatures as this would violate
    ///   the encapsulation boundary at the crate level.
    /// 
    /// Note: We don't need fine-grained visibility comparison here because
    /// the calling context (is_in_public_function_context) already filtered
    /// to only check public functions. For pub(crate) and other restricted
    /// visibility functions, this check is never invoked.
    fn is_field_visible_from_current_context(
        &self,
        field_def: &ty::FieldDef,
    ) -> bool {
        let field_visibility = self.tcx.visibility(field_def.did);
        match field_visibility {
            ty::Visibility::Public => true,
            ty::Visibility::Restricted(_) => {
                // Non-public fields (pub(crate), pub(super), private) cannot be
                // exposed in public functions. This maintains the crate boundary.
                false
            }
        }
    }

    /// Attempts to find a more precise span for a specific field name within a view span.
    /// 
    /// This provides better error highlighting by pointing to the specific field name
    /// rather than the entire view. Since HIR doesn't preserve individual field spans,
    /// we do a best-effort search within the view span's source text.
    fn find_field_span_in_view(&self, view_span: Span, field_name: Symbol) -> Option<Span> {
        let source_map = self.tcx.sess.source_map();
        let span_snippet = source_map.span_to_snippet(view_span).ok()?;
        
        let field_name_str = field_name.as_str();
        
        // Look for the field name in the view snippet
        // This is a simple approach - we could make it more sophisticated
        if let Some(pos) = span_snippet.find(field_name_str) {
            let start = view_span.lo() + rustc_span::BytePos(pos as u32);
            let end = start + rustc_span::BytePos(field_name_str.len() as u32);
            Some(Span::new(start, end, view_span.ctxt(), view_span.parent()))
        } else {
            None
        }
    }
}

// ============================================================================
// ERROR REPORTING
// ============================================================================

impl<'a, 'tcx> FnCtxt<'a, 'tcx> {
    /// Reports an error when a view is applied to an incompatible type.
    fn report_invalid_view_target_error(
        &self,
        hir_ty: &'tcx hir::Ty<'tcx>,
        target_def_id: DefId,
    ) -> ErrorGuaranteed {
        let mut err = self.dcx().struct_span_err(
            hir_ty.span,
            "only reference types to structs can be enriched with a view",
        );

        err.span_label(hir_ty.span, "view not allowed here");

        // Provide information about what type this actually is.
        let target_ty = self.tcx.type_of(target_def_id).instantiate_identity();
        match target_ty.kind() {
            ty::Adt(adt_def, _) if adt_def.is_enum() => {
                err.note("views cannot be applied to enums");
                err.help("consider applying the view to individual enum variant fields instead");
            }
            ty::Adt(adt_def, _) if adt_def.is_union() => {
                err.note("views cannot be applied to unions");
            }
            _ => {
                err.note(format!("views can only be used with structs, but this is a {:?}", target_ty.kind()));
            }
        }

        err.emit()
    }

    /// Reports an error when a view is applied to a non-named type (primitives, slices, etc).
    fn report_view_on_non_named_type_error(
        &self,
        hir_ty: &'tcx hir::Ty<'tcx>,
    ) -> ErrorGuaranteed {
        let mut err = self.dcx().struct_span_err(
            hir_ty.span,
            "only reference types to structs can be enriched with a view",
        );

        err.span_label(hir_ty.span, "view not allowed here");
        err.note("views can only be used with struct types");

        err.emit()
    }

    /// Reports an error when a field access path is not included in the view.
    fn report_field_path_not_in_view_error(
        &self,
        field_expr: &'tcx hir::Expr<'tcx>,
        access_path: &[Symbol],
        view_constraint: &ViewConstraint<'tcx>,
    ) -> ErrorGuaranteed {
        let access_path_str: Vec<_> = access_path.iter().map(|s| s.to_string()).collect();
        let access_str = access_path_str.join(".");

        let mut err = self.dcx().struct_span_err(
            field_expr.span,
            format!("field `{}` is not accessible through this view", access_str),
        );

        err.span_label(field_expr.span, "field access not allowed by view");
        err.span_label(view_constraint.span, "view defined here");

        // Add information about the constrained type
        if let Some(type_name) = self.tcx.opt_item_name(view_constraint.struct_def_id) {
            err.note(format!(
                "the view on type `{}` only allows access to specific field paths",
                type_name
            ));
        }

        // Show available field paths from the view.
        let available_paths: Vec<String> = view_constraint
            .view
            .fields
            .iter()
            .map(|field| {
                let path_strs: Vec<_> = field.path.iter().map(|s| s.to_string()).collect();
                path_strs.join(".")
            })
            .collect();

        if !available_paths.is_empty() {
            err.help(format!(
                "this view allows access to: {}",
                available_paths.join(", ")
            ));
            err.note("you can access these fields and any of their subfields");
        }

        err.emit()
    }

    /// Reports an error when view mutability doesn't match reference mutability.
    fn report_view_mutability_mismatch_error(
        &self,
        view: &hir::View<'tcx>,
        view_span: Span,
    ) -> ErrorGuaranteed {
        let mut err = self.dcx().struct_span_err(
            view_span,
            "view contains mutable fields but reference is not mutable",
        );

        err.span_label(view_span, "view with mutable fields");

        // Find and highlight the mutable fields
        for field in view.fields {
            if field.mutbl == hir::Mutability::Mut {
                if let Some(&field_name) = field.path.last() {
                    err.note(format!("field `{}` is marked as mutable in the view", field_name));
                }
            }
        }

        err.help("change the reference to `&mut` to allow mutable access to fields");
        err.note("views that grant mutable access to any field require a mutable reference");

        err.emit()
    }

    /// Reports an error when a view exposes private fields in a public context.
    fn report_view_privacy_violation_error(
        &self,
        view_field: &hir::ViewField<'tcx>,
        field_name: Symbol,
        def_id: DefId,
        view_span: Span,
    ) -> ErrorGuaranteed {
        // Try to find a more precise span for the specific field name
        let field_span = self.find_field_span_in_view(view_span, field_name).unwrap_or(view_span);

        let mut err = self.dcx().struct_span_err(
            field_span,
            format!("view cannot expose private field `{}` in public context", field_name),
        );

        err.span_label(field_span, "private field in view");
        
        if let Some(type_name) = self.tcx.opt_item_name(def_id) {
            err.note(format!(
                "field `{}` is private in struct `{}`",
                field_name, type_name
            ));
        }

        // Show the full path context if this is part of a nested path
        if view_field.path.len() > 1 {
            let path_str: Vec<_> = view_field.path.iter().map(|s| s.to_string()).collect();
            err.note(format!("in view field path: {}", path_str.join(".")));
        }

        err.help("remove private fields from the view or make the function private");
        err.note("public functions cannot expose private fields through views");

        err.emit()
    }

    /// Reports an error when a view references a field that doesn't exist.
    fn report_view_field_not_found_error(
        &self,
        view_field: &hir::ViewField<'tcx>,
        field_name: Symbol,
        def_id: DefId,
        view_span: Span,
    ) -> ErrorGuaranteed {
        // Try to find a more precise span for the specific field name.
        let field_span = self.find_field_span_in_view(view_span, field_name).unwrap_or(view_span);
        
        let mut err = self.dcx().struct_span_err(
            field_span,
            format!("field `{}` does not exist on struct", field_name),
        );

        err.span_label(field_span, "field referenced in view");

        // Try to suggest similar field names
        let struct_ty = self.tcx.type_of(def_id).instantiate_identity();
        if let ty::Adt(adt_def, _) = struct_ty.kind() {
            let available_fields: Vec<_> = adt_def
                .non_enum_variant()
                .fields
                .iter()
                .map(|field| field.name.to_string())
                .collect();

            if !available_fields.is_empty() {
                err.note(format!(
                    "available fields on this struct: {}",
                    available_fields.join(", ")
                ));
            }

            let available_field_symbols: Vec<Symbol> = adt_def
                .non_enum_variant()
                .fields
                .iter()
                .map(|field| field.name)
                .collect();
        
            // Try to find a similar field name using edit distance.
            if let Some(similar_field) = find_best_match_for_name(&available_field_symbols, field_name, None) {
                err.help(format!("did you mean `{}`?", similar_field));
            }
        }

        // If this is a nested path, clarify which part was problematic and show the full path.
        if view_field.path.len() > 1 {
            let path_str: Vec<_> = view_field.path.iter().map(|s| s.to_string()).collect();
            err.note(format!(
                "while resolving nested view field path: `{}`\n\
                the problematic component is: `{}`",
                path_str.join("."), field_name
            ));
        }

        err.emit()
    }

    /// Reports an error when a field in a view path is not a struct (can't have nested access).
    fn report_non_struct_field_in_view_path_error(
        &self,
        view_field: &hir::ViewField<'tcx>,
        field_ty: Ty<'tcx>,
        path_index: usize,
        view_span: Span,
    ) -> ErrorGuaranteed {
        let path_str: Vec<_> = view_field.path.iter().map(|s| s.to_string()).collect();
        let current_path = path_str[..=path_index].join(".");
        let next_component = view_field.path[path_index + 1];
        let problematic_path = path_str[..=path_index + 1].join(".");

        // Try to find a more precise span for the current path (the field that is not a struct)
        let problem_field = view_field.path[path_index];
        let field_span = self.find_field_span_in_view(view_span, problem_field).unwrap_or(view_span);

        let mut err = self.dcx().struct_span_err(
            field_span,
            format!(
                "cannot access `{}` on `{}` because it is not a struct",
                next_component, current_path
            ),
        );

        err.span_label(field_span, "not a struct field");

        err.note(format!(
            "`{}` has type `{:?}`, which does not support field access in views",
            current_path, field_ty.kind()
        ));

        err.help("only struct types support nested field access in views");

        err.note(format!(
            "while resolving view field path: `{}`\n\
            the problematic component is: `{}` (trying to access field `{}` on non-struct `{}`)",
            path_str.join("."), problematic_path, next_component, current_path
        ));

        err.emit()
    }

    /// Reports an error when a view field has an invalid path structure.
    fn report_invalid_view_field_path_error(
        &self,
        _view_field: &hir::ViewField<'tcx>,
        view_span: Span,
    ) -> ErrorGuaranteed {
        let mut err = self.dcx().struct_span_err(
            view_span,
            "invalid view field path",
        );

        err.span_label(view_span, "invalid field path");
        err.note("view field paths cannot be empty");

        err.emit()
    }

    /// Reports an error when view fields are not disjoint.
    fn report_view_fields_not_disjoint_error(
        &self,
        field1: &hir::ViewField<'tcx>,
        field2: &hir::ViewField<'tcx>,
        view_span: Span,
    ) -> ErrorGuaranteed {
        let path1_str: Vec<_> = field1.path.iter().map(|s| s.to_string()).collect();
        let path2_str: Vec<_> = field2.path.iter().map(|s| s.to_string()).collect();
        let path1 = path1_str.join(".");
        let path2 = path2_str.join(".");

        let mut err = self.dcx().struct_span_err(
            view_span,
            "view contains overlapping field paths",
        );

        err.span_label(view_span, "overlapping fields in view");

        // Determine the relationship between the paths.
        if path1 == path2 {
            err.note(format!("field `{}` appears multiple times in the view", path1));
            err.help("remove the duplicate field from the view");
        } else if field1.path.len() < field2.path.len() {
            err.note(format!(
                "field `{}` is a prefix of `{}`",
                path1, path2
            ));
            err.help(format!(
                "remove either `{}` or `{}` from the view - they overlap",
                path1, path2
            ));
        } else {
            err.note(format!(
                "field `{}` is a prefix of `{}`",
                path2, path1
            ));
            err.help(format!(
                "remove either `{}` or `{}` from the view - they overlap",
                path2, path1
            ));
        }

        err.note("all fields in a view must be disjoint (no field can be a prefix of another)");

        err.emit()
    }
}
