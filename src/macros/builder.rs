//! Declarative Query Builder Generator with Hybrid Validation and Preemptive AST Metrics.
//!
//! This module provides the [`define_query_builder!`](crate::define_query_builder) macro, which automates the creation
//! of strongly typed, fluent GraphQL query builders. It generates argument setter methods,
//! enforces declarative boundary validation, integrates with Type-State field selectors,
//! and compiles queries into either inline documents or parameterized GraphQL Variables.
//!
//! ### Supported Validation Directives
//!
//! | Directive | Target Types | Description |
//! | :--- | :--- | :--- |
//! | `#[validate(required)]` | Any | Enforces that the parameter must be called before dispatch. |
//! | `#[validate(range = a..=b)]` | Numbers | Ensures a numeric value falls within `RangeInclusive`. |
//! | `#[validate(min = n)]` | Numbers | Enforces a lower numeric bound (`val >= n`). |
//! | `#[validate(max = n)]` | Numbers | Enforces an upper numeric bound (`val <= n`). |
//! | `#[validate(min_len = n)]` | Strings / Lists | Enforces minimum element or character count. |
//! | `#[validate(max_len = n)]` | Strings / Lists | Enforces maximum element or character count ceiling. |
//! | `#[validate(custom = fn)]` | Any | Custom predicate: `fn(&T, &ArgMeta) -> Result<(), String>`. |
//!
//! ### Argument Representation Categories (`arg_kind`)
//!
//! 1. `scalar` — Primitive numbers and booleans (`u32`, `i64`, `bool`). Rendered without quotes inline.
//! 2. `string` — String arguments accepting `impl Into<String>`. Inlined with escaped quotes.
//! 3. `display` — Enums and formatted values wrapped in quotes when rendered inline.
//! 4. `r#enum` — Raw GraphQL schema enums (rendered without quotes inline: `order: ranked`).
//! 5. `list` — Collections accepting `impl IntoIterator<Item = S> where S: ToString`.

/// Declarative macro for generating strongly typed GraphQL query builders with hybrid validation.
///
/// Automates the construction of a type-safe fluent interface for schema root operations
/// (e.g. `Query.animes`, `Query.users`, `Query.character`), eliminating repetitive boilerplate
/// while enforcing strict client-side validation rules.
///
/// # Struct Attributes
///
/// * `#[root = root_name]` — *(Mandatory)* Name of the root field in the remote GraphQL schema.
/// * `#[selector = SelectorType]` — *(Mandatory)* Path to the associated Type-State field selector.
///
/// # Example
///
/// ```rust,no_run
/// use moongraphql_builder::prelude::*;
///
/// define_selector! {
///     #[default = "id name"]
///     pub struct AnimeSelector {
///         id: id,
///         name: name,
///     }
/// }
///
/// define_query_builder! {
///     /// Query builder for retrieving paginated anime records.
///     #[root = animes]
///     #[selector = AnimeSelector]
///     pub struct AnimesQueryBuilder {
///         /// Mandatory page number.
///         #[validate(required)]
///         #[validate(min = 1)]
///         page: u32 => page("PositiveInt", scalar),
///
///         /// Optional limit (1 to 50).
///         #[validate(range = 1..=50)]
///         limit: u32 => limit("PositiveInt", scalar),
///
///         /// Search filter query.
///         #[validate(min_len = 2)]
///         search: String => search("String", string),
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let query = AnimesQueryBuilder::new()
///     .page(1)
///     .limit(10)
///     .search("Bakemonogatari")
///     .select(|f| f.id().name());
///
/// println!("Compiled Query:\n{}", query._build_query());
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! define_query_builder {
    // -------------------------------------------------------------------------
    // Entry Point: Match struct declarations and initialize attribute parsing
    // -------------------------------------------------------------------------
    (
        $( #[$($struct_meta:tt)*] )*
        $vis:vis struct $name:ident {
            $(
                $( #[$($field_meta:tt)*] )*
                $arg_fn:ident : $arg_ty:ty => $gql_arg:ident ( $gql_type_str:literal, $arg_kind:ident )
            ),* $(,)?
        }
    ) => {
        $crate::define_query_builder! {
            @parse_struct_attrs
            struct = ($vis struct $name),
            raw_attrs = ( $( #[$($struct_meta)*] )* ),
            clean_docs = [ ],
            root_name = None,
            selector_ty = None,
            fields = [ $( ( ( $( #[$($field_meta)*] )* ), $arg_fn, $arg_ty, $gql_arg, $gql_type_str, $arg_kind ) )* ]
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Parse #[root = ...] attribute
    // -------------------------------------------------------------------------
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[root = $root_ident:ident] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        root_name = $old_root:tt,
        selector_ty = $selector_ty:tt,
        fields = $fields:tt
    ) => {
        $crate::define_query_builder! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* ],
            root_name = (Some($root_ident)),
            selector_ty = $selector_ty,
            fields = $fields
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Parse #[selector = ...] attribute
    // -------------------------------------------------------------------------
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[selector = $sel:path] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        root_name = $root_name:tt,
        selector_ty = $old_sel:tt,
        fields = $fields:tt
    ) => {
        $crate::define_query_builder! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* ],
            root_name = $root_name,
            selector_ty = (Some($sel)),
            fields = $fields
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Preserve doc comments and discard unrecognized attributes
    // -------------------------------------------------------------------------
    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[doc $($doc_tt:tt)*] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        root_name = $root_name:tt,
        selector_ty = $selector_ty:tt,
        fields = $fields:tt
    ) => {
        $crate::define_query_builder! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($docs)*] )* #[doc $($doc_tt)*] ],
            root_name = $root_name,
            selector_ty = $selector_ty,
            fields = $fields
        }
    };

    (
        @parse_struct_attrs
        struct = $struct_def:tt,
        raw_attrs = ( #[$($other:tt)*] $( #[$($rest:tt)*] )* ),
        clean_docs = $clean_docs:tt,
        root_name = $root_name:tt,
        selector_ty = $selector_ty:tt,
        fields = $fields:tt
    ) => {
        $crate::define_query_builder! {
            @parse_struct_attrs
            struct = $struct_def,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = $clean_docs,
            root_name = $root_name,
            selector_ty = $selector_ty,
            fields = $fields
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Finalize attribute parsing and invoke @build_struct
    // -------------------------------------------------------------------------
    (
        @parse_struct_attrs
        struct = ($vis:vis struct $name:ident),
        raw_attrs = (),
        clean_docs = [ $( #[$($docs:tt)*] )* ],
        root_name = (Some($root_ident:ident)),
        selector_ty = (Some($selector_ty:path)),
        fields = [ $( ( ( $( #[$($field_meta:tt)*] )* ), $arg_fn:ident, $arg_ty:ty, $gql_arg:ident, $gql_type_str:literal, $arg_kind:ident ) )* ]
    ) => {
        $crate::define_query_builder! {
            @build_struct
            docs = [ $( #[$($docs)*] )* ],
            vis = $vis,
            name = $name,
            root = $root_ident,
            selector = $selector_ty,
            fields = [ $( ( ( $( #[$($field_meta)*] )* ), $arg_fn, $arg_ty, $gql_arg, $gql_type_str, $arg_kind ) )* ]
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Code Generator (@build_struct)
    // -------------------------------------------------------------------------
    (
        @build_struct
        docs = [ $( #[$($doc_meta:tt)*] )* ],
        vis = $vis:vis,
        name = $name:ident,
        root = $root_ident:ident,
        selector = $selector_ty:path,
        fields = [ $( ( ( $( #[$($field_meta:tt)*] )* ), $arg_fn:ident, $arg_ty:ty, $gql_arg:ident, $gql_type_str:literal, $arg_kind:ident ) )* ]
    ) => {
        $( #[$($doc_meta)*] )*
        #[derive(Debug, Default, Clone)]
        $vis struct $name {
            $(
                pub(crate) $arg_fn: Option<$arg_ty>,
            )*
            pub(crate) selector_depth: u32,
            pub(crate) selector_complexity: u32,
            pub(crate) fields_str: String,
            pub(crate) max_complexity_override: Option<u32>,
            pub(crate) max_depth_override: Option<u32>,
        }

        impl $name {
            /// Instantiates a new, empty query builder instance with default settings.
            pub fn new() -> Self {
                Self::default()
            }

            // Generate fluent setter methods for each argument
            $(
                $crate::define_query_builder! {
                    @emit_setter
                    fn_name = $arg_fn,
                    ty = $arg_ty,
                    kind = $arg_kind,
                    raw_attrs = ( $( #[$($field_meta)*] )* ),
                    clean_docs = []
                }
            )*

            /// Binds and evaluates a field selection closure using the associated Type-State selector.
            ///
            /// # Arguments
            ///
            /// * `selector_fn` (`F`) — Selection closure receiving an initial Type-State selector and returning the selection.
            ///
            /// # Returns
            ///
            /// The updated query builder instance.
            pub fn select<F, S>(mut self, selector_fn: F) -> Self
            where
                F: FnOnce($selector_ty) -> S,
                S: $crate::traits::BuildableSelector,
            {
                let initial_selector = <$selector_ty>::new();
                let built_selector = selector_fn(initial_selector);
                self.fields_str = $crate::traits::BuildableSelector::build(&built_selector);
                self.selector_complexity = $crate::traits::BuildableSelector::complexity(&built_selector);
                self.selector_depth = $crate::traits::BuildableSelector::depth(&built_selector);
                self
            }

            /// Explicitly overrides the local maximum theoretical complexity budget for this query.
            ///
            /// # Arguments
            ///
            /// * `max` (`u32`) — Maximum allowable complexity budget.
            ///
            /// # Returns
            ///
            /// The updated query builder instance.
            pub fn with_max_complexity(mut self, max: u32) -> Self {
                self.max_complexity_override = Some(max);
                self
            }

            /// Explicitly overrides the local maximum theoretical nesting depth ceiling for this query.
            ///
            /// # Arguments
            ///
            /// * `max` (`u32`) — Maximum allowable tree depth ceiling.
            ///
            /// # Returns
            ///
            /// The updated query builder instance.
            pub fn with_max_depth(mut self, max: u32) -> Self {
                self.max_depth_override = Some(max);
                self
            }

            /// Internal helper for preemptive argument validation.
            #[doc(hidden)]
            pub fn _validate_args(&self) -> $crate::error::Result<()> {
                $(
                    let meta = $crate::validation::ArgMeta {
                        arg_name: stringify!($arg_fn),
                        gql_arg_name: stringify!($gql_arg),
                        root_query: stringify!($root_ident),
                    };

                    $crate::define_query_builder! {
                        @check_required
                        self.$arg_fn,
                        meta,
                        ( $( #[$($field_meta)*] )* )
                    }
                )*

                $(
                    if let Some(ref val) = self.$arg_fn {
                        let meta = $crate::validation::ArgMeta {
                            arg_name: stringify!($arg_fn),
                            gql_arg_name: stringify!($gql_arg),
                            root_query: stringify!($root_ident),
                        };
                        $crate::define_query_builder! {
                            @validate_field
                            val,
                            meta,
                            ( $( #[$($field_meta)*] )* )
                        }
                    }
                )*

                Ok(())
            }

            /// Compiles query components into body strings, variable declarations, and JSON values.
            #[doc(hidden)]
            pub fn _build_query_parts(
                &self,
                use_variables: bool,
            ) -> (
                String,
                Vec<String>,
                serde_json::Map<String, serde_json::Value>,
            ) {
                let mut args: Vec<String> = Vec::new();
                let mut decls: Vec<String> = Vec::new();
                let mut vars = serde_json::Map::new();

                $(
                    if use_variables {
                        if let Some(ref val) = self.$arg_fn {
                            let var_name = format!("{}_{}", stringify!($root_ident), stringify!($arg_fn));
                            args.push(format!("{}: ${}", stringify!($gql_arg), var_name));
                            decls.push(format!("${}: {}", var_name, $gql_type_str));
                            let json_val = $crate::define_query_builder!(@to_json_val val, $arg_kind);
                            vars.insert(var_name, json_val);
                        }
                    } else {
                        if let Some(ref val) = self.$arg_fn {
                            let inline_str = $crate::define_query_builder!(@to_inline_str stringify!($gql_arg), val, $arg_kind);
                            args.push(inline_str);
                        }
                    }
                )*

                let query_args = if args.is_empty() {
                    String::new()
                } else {
                    format!("({})", args.join(", "))
                };

                let selected_fields = if self.fields_str.is_empty() {
                    let selector_ty_as_crate = <$selector_ty as $crate::traits::BuildableSelector>::build(&<$selector_ty>::default());
                    if selector_ty_as_crate.is_empty() {
                        String::new()
                    } else {
                        format!(" {{ {} }}", selector_ty_as_crate)
                    }
                } else {
                    format!(" {{ {} }}", self.fields_str)
                };

                let body = format!("{}{}{}", stringify!($root_ident), query_args, selected_fields);
                (body, decls, vars)
            }

            /// Compiles a self-contained inline GraphQL query representation (`query { ... }`).
            pub fn _build_query(&self) -> String {
                let (q, _, _) = self._build_query_parts(false);
                format!("query {{ {} }}", q)
            }
        }

        // Implement AST metric inspection trait
        impl $crate::traits::GqlInspectable for $name {
            fn complexity(&self) -> u32 {
                if self.selector_complexity == 0 {
                    <$selector_ty as $crate::traits::BuildableSelector>::complexity(&<$selector_ty>::new())
                } else {
                    self.selector_complexity
                }
            }

            fn depth(&self) -> u32 {
                if self.selector_depth == 0 {
                    <$selector_ty as $crate::traits::BuildableSelector>::depth(&<$selector_ty>::new())
                } else {
                    self.selector_depth
                }
            }

            fn max_complexity(&self) -> Option<u32> {
                self.max_complexity_override
            }

            fn max_depth(&self) -> Option<u32> {
                self.max_depth_override
            }
        }

        // Implement preemptive argument validation trait
        impl $crate::traits::GqlValidatable for $name {
            fn validate_args(&self) -> $crate::error::Result<()> {
                self._validate_args()
            }
        }

        // Implement buildable query trait
        impl $crate::traits::BuildableQuery for $name {
            fn _build_query_parts(
                &self,
                use_variables: bool,
            ) -> (
                String,
                Vec<String>,
                serde_json::Map<String, serde_json::Value>,
            ) {
                self._build_query_parts(use_variables)
            }
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Check #[validate(required)]
    // -------------------------------------------------------------------------
    (
        @check_required
        $opt_val:expr,
        $meta:expr,
        ( #[validate(required)] $( #[$($rest:tt)*] )* )
    ) => {
        if $opt_val.is_none() {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-required", "arg" => $meta.arg_name, "query" => $meta.root_query)
            ));
        }
    };

    (
        @check_required
        $opt_val:expr,
        $meta:expr,
        ( #[$($other:tt)*] $( #[$($rest:tt)*] )* )
    ) => {
        $crate::define_query_builder! {
            @check_required
            $opt_val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @check_required
        $opt_val:expr,
        $meta:expr,
        ()
    ) => {};

    // -------------------------------------------------------------------------
    // Internal: Emit Argument Setter Methods (@emit_setter)
    // -------------------------------------------------------------------------
    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = $arg_kind:ident,
        raw_attrs = ( #[validate($($v:tt)*)] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $crate::define_query_builder! {
            @emit_setter
            fn_name = $arg_fn,
            ty = $arg_ty,
            kind = $arg_kind,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($clean)*] )* ]
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = $arg_kind:ident,
        raw_attrs = ( #[doc $($doc_tt:tt)*] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $crate::define_query_builder! {
            @emit_setter
            fn_name = $arg_fn,
            ty = $arg_ty,
            kind = $arg_kind,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($clean)*] )* #[doc $($doc_tt)*] ]
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = $arg_kind:ident,
        raw_attrs = ( #[$($other:tt)*] $( #[$($rest:tt)*] )* ),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $crate::define_query_builder! {
            @emit_setter
            fn_name = $arg_fn,
            ty = $arg_ty,
            kind = $arg_kind,
            raw_attrs = ( $( #[$($rest)*] )* ),
            clean_docs = [ $( #[$($clean)*] )* ]
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = scalar,
        raw_attrs = (),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $( #[$($clean)*] )*
        pub fn $arg_fn(mut self, val: $arg_ty) -> Self {
            self.$arg_fn = Some(val);
            self
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = string,
        raw_attrs = (),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $( #[$($clean)*] )*
        pub fn $arg_fn(mut self, val: impl Into<String>) -> Self {
            self.$arg_fn = Some(val.into());
            self
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = display,
        raw_attrs = (),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $( #[$($clean)*] )*
        pub fn $arg_fn(mut self, val: impl Into<$arg_ty>) -> Self {
            self.$arg_fn = Some(val.into());
            self
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = r#enum,
        raw_attrs = (),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $( #[$($clean)*] )*
        pub fn $arg_fn(mut self, val: impl Into<$arg_ty>) -> Self {
            self.$arg_fn = Some(val.into());
            self
        }
    };

    (
        @emit_setter
        fn_name = $arg_fn:ident,
        ty = $arg_ty:ty,
        kind = list,
        raw_attrs = (),
        clean_docs = [ $( #[$($clean:tt)*] )* ]
    ) => {
        $( #[$($clean)*] )*
        pub fn $arg_fn<I, S>(mut self, items: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: ToString,
        {
            self.$arg_fn = Some(items.into_iter().map(|it| it.to_string()).collect());
            self
        }
    };

    // -------------------------------------------------------------------------
    // Internal: Validate Individual Rules (@validate_field)
    // -------------------------------------------------------------------------
    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(required)] $( #[$($rest:tt)*] )* )
    ) => {
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(range = $range:expr)] $( #[$($rest:tt)*] )* )
    ) => {
        if !$range.contains($val) {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-range", "arg" => $meta.arg_name, "query" => $meta.root_query, "range" => format!("{:?}", $range), "got" => format!("{:?}", $val))
            ));
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(min = $min:expr)] $( #[$($rest:tt)*] )* )
    ) => {
        if *$val < $min {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-min", "arg" => $meta.arg_name, "query" => $meta.root_query, "min" => $min, "got" => $val))
            );
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(max = $max:expr)] $( #[$($rest:tt)*] )* )
    ) => {
        if *$val > $max {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-max", "arg" => $meta.arg_name, "query" => $meta.root_query, "max" => $max, "got" => $val))
            );
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(min_len = $len:expr)] $( #[$($rest:tt)*] )* )
    ) => {
        if $val.len() < $len {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-min-len", "arg" => $meta.arg_name, "query" => $meta.root_query, "len" => $len, "got" => $val.len()))
            );
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(max_len = $len:expr)] $( #[$($rest:tt)*] )* )
    ) => {
        if $val.len() > $len {
            return Err($crate::error::MoongqlError::ValidationError(
                $crate::tr!("err-val-max-len", "arg" => $meta.arg_name, "query" => $meta.root_query, "len" => $len, "got" => $val.len()))
            );
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[validate(custom = $func:path)] $( #[$($rest:tt)*] )* )
    ) => {
        if let Err(e) = $func($val, &$meta) {
            return Err($crate::error::MoongqlError::ValidationError(e.to_string()));
        }
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ( #[$($other:tt)*] $( #[$($rest:tt)*] )* )
    ) => {
        $crate::define_query_builder! {
            @validate_field
            $val,
            $meta,
            ( $( #[$($rest)*] )* )
        }
    };

    (
        @validate_field
        $val:expr,
        $meta:expr,
        ()
    ) => {};

    // -------------------------------------------------------------------------
    // Internal: Convert Values to JSON (@to_json_val)
    // -------------------------------------------------------------------------
    (@to_json_val $val:expr, scalar) => { serde_json::json!(*$val) };
    (@to_json_val $val:expr, string) => { serde_json::Value::String($val.clone()) };
    (@to_json_val $val:expr, display) => { serde_json::Value::String($val.to_string()) };
    (@to_json_val $val:expr, r#enum) => { serde_json::Value::String($val.to_string()) };
    (@to_json_val $val:expr, list) => {
        serde_json::Value::Array($val.iter().map(|it| serde_json::Value::String(it.clone())).collect())
    };

    // -------------------------------------------------------------------------
    // Internal: Convert Values to Inline Strings (@to_inline_str)
    // -------------------------------------------------------------------------
    (@to_inline_str $arg_name:expr, $val:expr, scalar) => { format!("{}: {}", $arg_name, $val) };
    (@to_inline_str $arg_name:expr, $val:expr, string) => { format!("{}: \"{}\"", $arg_name, $val) };
    (@to_inline_str $arg_name:expr, $val:expr, display) => { format!("{}: \"{}\"", $arg_name, $val) };
    (@to_inline_str $arg_name:expr, $val:expr, r#enum) => { format!("{}: {}", $arg_name, $val) };
    (@to_inline_str $arg_name:expr, $val:expr, list) => {
        format!("{}: [{}]", $arg_name, $val.iter().map(|it| format!("\"{}\"", it)).collect::<Vec<_>>().join(", "))
    };
}

// =============================================================================
// 🗄️ SECTION 8: [ARCHIVE] STATIC COMPILE-TIME CONST GENERICS TYPE-STATE PROTOTYPE
// =============================================================================
//
// Preserved prototype implementation of complete compile-time mandatory field enforcement
// utilizing Const Generics state flags (`const $arg_fn: bool = false`).
//
// Preserved for reference: the rustc compiler emits unhelpful diagnostic traces
// such as `the trait BuildableQuery is not implemented for Builder<false, true, ...>`,
// whereas the runtime validation via `_validate_args()` above yields localized,
// perfectly clear error diagnostics pointing out the exact missing argument name.
//
// To reactivate the compile-time Type-State generator in the future:
// 1. Replace Section 3 with generic struct generation `struct $name< $( const $arg_fn: bool = false ),* >`.
// 2. Uncomment helper patterns `@classify_fields`, `@split_generics`, and `@emit_setter_method`.
//
/*
macro_rules! _type_state_query_builder_archive {
    // 1. Classify fields into required / optional
    (
        @classify_fields
        raw_fields = [ ( $f_attrs:tt, $arg_fn:ident, $arg_ty:ty, $gql_arg:ident, $gql_type_str:literal, $arg_kind:ident ) $( $rest:tt )* ],
        classified = [ $( $c_fields:tt )* ]
    ) => {
        // Evaluate presence of #[validate(required)] in $f_attrs -> emits (true) or (false)
    };

    // 2. Split generics for BuildableQuery trait implementation
    // Mandatory arguments enforce `<true>`, optional ones accept `<{ $opt }>`
    (
        @split_generics
        classified = [ ( (true), $f_meta:tt, $arg_fn:ident, $arg_ty:ty, $gql_arg:ident, $gql_type:literal, $kind:ident ) $( $rest:tt )* ],
        impl_generics = [ $( $impl_gen:tt )* ],
        type_args = [ $( $type_arg:tt )* ]
    ) => {
        // Inject true into type_args
    };
}
*/
