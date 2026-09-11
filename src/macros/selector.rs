//! Declarative Type-State Field Selector Generator.
//!
//! This module provides the [`define_selector!`](crate::define_selector) macro, expanding declarative
//! field selection sets into strongly typed selectors powered by the Type-State
//! pattern and Const Generics. The macro guarantees compile-time field deduplication,
//! automated closure type inference, deterministic AST serialization, and preemptive
//! calculation of AST complexity and depth metrics.

/// Declarative macro for generating Type-State GraphQL field selectors with compile-time deduplication,
/// full closure type inference, and an integrated AST complexity/depth calculator.
///
/// # Architectural Highlights
///
/// - **Compile-Time Type-State**: Every struct field is represented by an independent state flag
///   `const field_name: bool`. Invoking the same field method twice on a single node is
///   physically prohibited by the Rust compiler during static analysis.
/// - **100% Closure Type Inference**: In nested selector calls (e.g., `.poster(|p| p.original_url())`),
///   the closure argument `p` is inferred by the compiler automatically without requiring explicit type annotations.
/// - **Deterministic Canonical Output**: Compiled fields are sorted alphabetically via [`std::collections::BTreeSet`],
///   guaranteeing absolute AST stability, reproducible query hashes, and optimal HTTP proxy cache hits.
/// - **Preemptive Complexity and Depth Calculation**: Recursively accounts for base node weights
///   and child field scores, preventing schema limit violations before network transmission.
///
/// # Struct Attributes (Selector Manifest)
///
/// - `#[default = "field1 field2 ..."]` — Default whitespace-separated fieldset returned
///   if the developer does not invoke any selector methods explicitly.
/// - `#[default_complexity = N]` — Base theoretical complexity weight of the root node (defaults to `1` if omitted).
///
/// # Field Attributes
///
/// - `#[complexity = N]` — Explicit override for field complexity weight (e.g., for heavy
///   relations such as `related`, `user_rate`, `screenshots`).
/// - Scalar field syntax: `field_name: gqlFieldName`
/// - Nested object syntax: `field_name(NestedSelectorType): gqlFieldName`
///
/// # Example
///
/// ```rust,no_run
/// use moongraphql_builder::prelude::*;
///
/// // 1. Nested child selector
/// define_selector! {
///     /// Selector for poster image assets.
///     #[default_complexity = 5]
///     #[default = "originalUrl mainUrl"]
///     pub struct PosterSelector {
///         #[complexity = 1]
///         id: id,
///         #[complexity = 1]
///         original_url: originalUrl,
///         #[complexity = 1]
///         main_url: mainUrl,
///     }
/// }
///
/// // 2. Root entity selector with deduplication
/// define_selector! {
///     /// Anime field selector with compile-time deduplication.
///     #[default_complexity = 1]
///     #[default = "id name russian"]
///     pub struct ExampleSelector {
///         #[complexity = 1]
///         id: id,
///         #[complexity = 1]
///         name: name,
///         #[complexity = 1]
///         russian: russian,
///         #[complexity = 5]
///         poster(PosterSelector): poster,
///     }
/// }
///
/// let selector = ExampleSelector::new()
///     .id()
///     .name()
///     .poster(|p| p.original_url());
///
/// assert_eq!(selector.build(), "id name poster { originalUrl }");
/// assert_eq!(selector.complexity(), 1 + 1 + 1 + 5 + 1); // 9 points
/// assert_eq!(selector.depth(), 2);
/// ```
#[macro_export]
macro_rules! define_selector {
    // =========================================================================
    // 🚪 SECTION 1: UNIFIED ENTRYPOINT AND TT-MUNCHER INITIALIZATION
    // =========================================================================
    (
        $( #[$($struct_meta:tt)*] )*
        $vis:vis struct $name:ident {
            $(
                $( #[$($field_meta:tt)*] )*
                $field_fn:ident $( ($nested_ty:ident) )? : $gql_ident:ident
            ),* $(,)?
        }
    ) => {
        $crate::define_selector! {
            @parse_struct_attrs
            struct = ($vis struct $name),
            raw_attrs = ( $( #[$($struct_meta)*] )* ),
            clean_docs = [],
            default_str = None,
            default_cost = None,
            fields = [
                $(
                    ( ( $( #[$($field_meta)*] )* ), $field_fn, ( $($nested_ty)? ), $gql_ident )
                )*
            ]
        }
    };

    // =========================================================================
    // 🧮 SECTION 2: COMPLEXITY RESOLUTION HELPERS
    // =========================================================================

    // Unwrap optional complexity score with fallback default
    (@unwrap_cost None, $default:expr) => { $default };
    (@unwrap_cost (Some($cost:expr)), $default:expr) => { $cost };

    // Scalar field without explicit #[complexity]: default baseline cost = 1
    (@field_cost () , () ) => { 1 };

    // Nested field without explicit #[complexity]: inherits DEFAULT_COMPLEXITY of the child selector
    (@field_cost () , ($nested_ty:path) ) => {
        <$nested_ty as $crate::traits::BuildableSelector>::DEFAULT_COMPLEXITY
    };

    // Field annotated with explicit #[complexity = N]
    (@field_cost ( #[complexity = $c:expr] $( #[$($rest:tt)*] )* ) , $nested:tt ) => { $c };

    // Discard extraneous attributes when searching for #[complexity]
    (@field_cost ( #[$($other:tt)*] $( #[$($rest:tt)*] )* ) , $nested:tt ) => {
        $crate::define_selector!(@field_cost ( $( #[$($rest)*] )* ) , $nested )
    };

    // =========================================================================
    // ⚙️ SECTION 3: STRUCT ATTRIBUTE TT-MUNCHER (#[default], #[default_complexity])
    // =========================================================================

    // 1. Detect base node complexity attribute: #[default_complexity = N]
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[default_complexity = $cost:expr] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        default_str = $default_str:tt,
        default_cost = $old_cost:tt,
        fields = $fields:tt
    ) => {
        $crate::define_selector! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* ],
            default_str = $default_str,
            default_cost = (Some($cost)),
            fields = $fields
        }
    };

    // 2. Detect default fieldset string attribute: #[default = "..."]
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[default = $default:expr] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        default_str = $default_str:tt,
        default_cost = $default_cost:tt,
        fields = $fields:tt
    ) => {
        $crate::define_selector! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* ],
            default_str = (Some($default)),
            default_cost = $default_cost,
            fields = $fields
        }
    };

    // 3. Preserve doc comments /// and system metadata attributes
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[$($other:tt)*] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        default_str = $default_str:tt,
        default_cost = $default_cost:tt,
        fields = $fields:tt
    ) => {
        $crate::define_selector! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* #[$($other)*] ],
            default_str = $default_str,
            default_cost = $default_cost,
            fields = $fields
        }
    };

    // 4A. Attribute parsing complete (both default and default_complexity provided)
    (
        @parse_struct_attrs
        struct = ($vis:vis struct $name:ident),
        raw_attrs = (),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        default_str = (Some($default_str:expr)),
        default_cost = (Some($default_cost:expr)),
        fields = $fields:tt
    ) => {
        $crate::define_selector! {
            @build_struct
            default = $default_str,
            default_cost = $default_cost,
            docs = [ $( #[$($docs)*] )* ],
            struct = ($vis struct $name),
            fields = $fields
        }
    };

    // 4B. Attribute parsing complete (default_complexity omitted -> defaults to 1)
    (
        @parse_struct_attrs
        struct = ($vis:vis struct $name:ident),
        raw_attrs = (),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        default_str = (Some($default_str:expr)),
        default_cost = None,
        fields = $fields:tt
    ) => {
        $crate::define_selector! {
            @build_struct
            default = $default_str,
            default_cost = 1,
            docs = [ $( #[$($docs)*] )* ],
            struct = ($vis struct $name),
            fields = $fields
        }
    };

    // =========================================================================
    // 🏗️ SECTION 4: STRUCT GENERATION AND BUILDABLESELECTOR TRAIT IMPLEMENTATION
    // =========================================================================
    (
        @build_struct
        default = $default_str:expr,
        default_cost = $default_cost:expr,
        docs = [ $( #[$($doc_meta:tt)*] )* ],
        struct = ($vis:vis struct $name:ident),
        fields = [
            $(
                ( $f_meta:tt, $field_fn:ident, $f_nested:tt, $gql_ident:ident )
            )*
        ]
    ) => {
        $( #[$($doc_meta)*] )*
        #[doc = ""]
        #[doc = concat!("**Complexity**: `", stringify!($default_cost), "` (base root weight).")]
        #[allow(non_upper_case_globals)]
        #[derive(Debug, Clone)]
        $vis struct $name<
            $( const $field_fn: bool = false ),*
        > {
            /// Set of selected fields, sorted alphabetically in canonical order.
            pub(crate) fields: std::collections::BTreeSet<String>,
            /// Cumulative complexity weight score of currently selected fields.
            pub(crate) total_complexity: u32,
            /// Maximum AST nesting depth of the currently selected subtree.
            pub(crate) total_depth: u32,
        }

        impl $name {
            /// Instantiates a new empty Type-State selector in initial state (all field flags set to `false`).
            ///
            /// # Returns
            ///
            /// An initial selector with all field flags set to `false`.
            pub fn new() -> Self {
                Self {
                    fields: std::collections::BTreeSet::new(),
                    total_complexity: 0,
                    total_depth: 1,
                }
            }
        }

        impl Default for $name {
            /// Returns a new default-initialized selector instance.
            fn default() -> Self {
                Self::new()
            }
        }

        #[allow(non_upper_case_globals)]
        impl<
            $( const $field_fn: bool ),*
        > $name< $( { $field_fn } ),* > {
            /// Dynamically resolves the theoretical complexity weight of a field by its GraphQL schema identifier.
            ///
            /// # Arguments
            ///
            /// * `gql_name` (`&str`) — Name of the field as defined in the remote GraphQL schema.
            ///
            /// # Returns
            ///
            /// The complexity score assigned to the field.
            pub fn field_default_complexity(gql_name: &str) -> u32 {
                match gql_name {
                    $(
                        stringify!($gql_ident) => {
                            $crate::define_selector!(@field_cost $f_meta, $f_nested)
                        }
                    )*
                    _ => 1,
                }
            }
        }

        #[allow(non_upper_case_globals)]
        impl<
            $( const $field_fn: bool ),*
        > $crate::traits::BuildableSelector for $name< $( { $field_fn } ),* > {
            const DEFAULT_COMPLEXITY: u32 = $default_cost;

            fn build(&self) -> String {
                if self.fields.is_empty() {
                    return $default_str.to_string();
                }
                self.fields.iter().cloned().collect::<Vec<_>>().join(" ")
            }

            fn root_complexity(&self) -> u32 {
                Self::DEFAULT_COMPLEXITY
            }

            fn fields_complexity(&self) -> u32 {
                if self.fields.is_empty() {
                    let mut total = 0;
                    for field_name in $default_str.split_whitespace() {
                        total += Self::field_default_complexity(field_name);
                    }
                    total
                } else {
                    self.total_complexity
                }
            }

            fn depth(&self) -> u32 {
                if self.fields.is_empty() {
                    1 // Default scalar fieldset has nesting depth of 1
                } else {
                    self.total_depth
                }
            }
        }

        // Launch recursive TT-muncher to generate Type-State field methods
        $crate::define_selector! {
            @munch_all
            struct = $name,
            before = [],
            after = [
                $(
                    ( $f_meta, $field_fn, $f_nested, $gql_ident )
                )*
            ]
        }
    };

    // =========================================================================
    // 🔁 SECTION 5: RECURSIVE TYPE-STATE FIELD METHODS TT-MUNCHER
    // =========================================================================

    // Muncher base recursion stop condition
    (@munch_all struct = $name:ident, before = [ $( $b:tt )* ], after = []) => {};

    // Muncher step: partition flags into `before` and `after` lists around current field
    (
        @munch_all
        struct = $name:ident,
        before = [ $( ( $b_meta:tt, $b_fn:ident, $b_nested:tt, $b_gql:ident ) )* ],
        after = [
            ( $curr_meta:tt, $curr_fn:ident, $curr_nested:tt, $curr_gql:ident )
            $( ( $a_meta:tt, $a_fn:ident, $a_nested:tt, $a_gql:ident ) )*
        ]
    ) => {
        $crate::define_selector! {
            @parse_attrs
            struct = $name,
            before = [ $( $b_fn )* ],
            after = [ $( $a_fn )* ],
            field = $curr_fn,
            nested = $curr_nested,
            gql = $curr_gql,
            attrs = $curr_meta,
            clean_attrs = [],
            current_cost = None
        }

        $crate::define_selector! {
            @munch_all
            struct = $name,
            before = [
                $( ( $b_meta, $b_fn, $b_nested, $b_gql ) )*
                ( $curr_meta, $curr_fn, $curr_nested, $curr_gql )
            ],
            after = [ $( ( $a_meta, $a_fn, $a_nested, $a_gql ) )* ]
        }
    };

    // --- Extract #[complexity = N] from individual field attributes ---

    // 1. Detect #[complexity = N] attribute
    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = $nested:tt,
        gql = $curr_gql:ident,
        attrs = ( #[complexity = $cost:expr] $( #[$($rest:tt)*] )* ),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = $old_cost:tt
    ) => {
        $crate::define_selector! {
            @parse_attrs
            struct = $name,
            before = [ $( $b )* ],
            after = [ $( $a )* ],
            field = $curr_fn,
            nested = $nested,
            gql = $curr_gql,
            attrs = ( $( #[$($rest)*] )* ),
            clean_attrs = [ $( #[$($clean)*] )* ],
            current_cost = (Some($cost))
        }
    };

    // 2. Preserve field doc comments ///
    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = $nested:tt,
        gql = $curr_gql:ident,
        attrs = ( #[$($other:tt)*] $( #[$($rest:tt)*] )* ),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = $cost_opt:tt
    ) => {
        $crate::define_selector! {
            @parse_attrs
            struct = $name,
            before = [ $( $b )* ],
            after = [ $( $a )* ],
            field = $curr_fn,
            nested = $nested,
            gql = $curr_gql,
            attrs = ( $( #[$($rest)*] )* ),
            clean_attrs = [ $( #[$($clean)*] )* #[$($other)*] ],
            current_cost = $cost_opt
        }
    };

    // =========================================================================
    // 🚀 SECTION 6: SETTER EMISSION (SCALAR AND NESTED OBJECT FIELDS)
    // =========================================================================

    // 3A. Scalar field (nested = ()) -> transitions flag from false to true, depth ceiling = 1
    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = (),
        gql = $curr_gql:ident,
        attrs = (),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = (Some($cost:expr))
    ) => {
        #[allow(non_upper_case_globals)]
        impl<
            $( const $b: bool, )*
            $( const $a: bool, )*
        > $name< $( { $b }, )* false, $( { $a }, )* > {
            $( #[$($clean)*] )*
            #[doc = ""]
            #[doc = concat!("**Complexity**: `", stringify!($cost), "`")]
            pub fn $curr_fn(mut self) -> $name< $( { $b }, )* true, $( { $a }, )* > {
                self.fields.insert(stringify!($curr_gql).to_string());
                self.total_complexity += $cost;
                self.total_depth = self.total_depth.max(1);
                $name {
                    fields: self.fields,
                    total_complexity: self.total_complexity,
                    total_depth: self.total_depth,
                }
            }
        }
    };

    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = (),
        gql = $curr_gql:ident,
        attrs = (),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = None
    ) => {
        #[allow(non_upper_case_globals)]
        impl<
            $( const $b: bool, )*
            $( const $a: bool, )*
        > $name< $( { $b }, )* false, $( { $a }, )* > {
            $( #[$($clean)*] )*
            #[doc = ""]
            #[doc = "**Complexity**: `1`"]
            pub fn $curr_fn(mut self) -> $name< $( { $b }, )* true, $( { $a }, )* > {
                self.fields.insert(stringify!($curr_gql).to_string());
                self.total_complexity += 1;
                self.total_depth = self.total_depth.max(1);
                $name {
                    fields: self.fields,
                    total_complexity: self.total_complexity,
                    total_depth: self.total_depth,
                }
            }
        }
    };

    // 3B. Nested object (nested = ($nested_ty:ident)) -> 100% closure inference, depth = 1 + child_depth
    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = ($nested_ty:ident),
        gql = $curr_gql:ident,
        attrs = (),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = (Some($cost:expr))
    ) => {
        #[allow(non_upper_case_globals)]
        impl<
            $( const $b: bool, )*
            $( const $a: bool, )*
        > $name< $( { $b }, )* false, $( { $a }, )* > {
            $( #[$($clean)*] )*
            #[doc = ""]
            #[doc = concat!("**Complexity**: `", stringify!($cost), "`")]
            pub fn $curr_fn<F, S>(mut self, selector_fn: F) -> $name< $( { $b }, )* true, $( { $a }, )* >
            where
                F: FnOnce($nested_ty) -> S,
                S: $crate::traits::BuildableSelector,
            {
                let initial = <$nested_ty>::new();
                let built = selector_fn(initial);
                let child_cost = built.fields_complexity();
                let child_depth = built.depth();

                self.fields.insert(format!("{} {{ {} }}", stringify!($curr_gql), built.build()));
                self.total_complexity += $cost + child_cost;
                self.total_depth = self.total_depth.max(1 + child_depth);

                $name {
                    fields: self.fields,
                    total_complexity: self.total_complexity,
                    total_depth: self.total_depth,
                }
            }
        }
    };

    (
        @parse_attrs
        struct = $name:ident,
        before = [ $( $b:ident )* ],
        after = [ $( $a:ident )* ],
        field = $curr_fn:ident,
        nested = ($nested_ty:ident),
        gql = $curr_gql:ident,
        attrs = (),
        clean_attrs = [ $( #[$($clean:tt)*] )* ],
        current_cost = None
    ) => {
        #[allow(non_upper_case_globals)]
        impl<
            $( const $b: bool, )*
            $( const $a: bool, )*
        > $name< $( { $b }, )* false, $( { $a }, )* > {
            $( #[$($clean)*] )*
            #[doc = ""]
            #[doc = concat!("**Complexity**: Base weight of [`", stringify!($nested_ty), "`].")]
            pub fn $curr_fn<F, S>(mut self, selector_fn: F) -> $name< $( { $b }, )* true, $( { $a }, )* >
            where
                F: FnOnce($nested_ty) -> S,
                S: $crate::traits::BuildableSelector,
            {
                let field_cost = <$nested_ty as $crate::traits::BuildableSelector>::DEFAULT_COMPLEXITY;
                let initial = <$nested_ty>::new();
                let built = selector_fn(initial);
                let child_cost = built.fields_complexity();
                let child_depth = built.depth();

                self.fields.insert(format!("{} {{ {} }}", stringify!($curr_gql), built.build()));
                self.total_complexity += field_cost + child_cost;
                self.total_depth = self.total_depth.max(1 + child_depth);

                $name {
                    fields: self.fields,
                    total_complexity: self.total_complexity,
                    total_depth: self.total_depth,
                }
            }
        }
    };
}
