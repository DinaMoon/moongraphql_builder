//! Declarative Composite Document Coordinator Generator.
//!
//! This module provides the [`define_composite_query!`](crate::define_composite_query) macro, which expands
//! multi-root GraphQL document coordinators into high-performance structures supporting
//! batch validation, cumulative AST complexity summation, tree depth evaluation, and seamless
//! variable merging across disparate operations within a single network round-trip (1 RTT).

/// Declarative macro for generating composite multi-root GraphQL document coordinators.
///
/// Bundles multiple independent query operations (e.g. `Query.viewer`, `Query.animes`, `Query.mangas`)
/// into a unified cohesive document executed over the wire in a **single network round-trip (1 RTT)**.
///
/// # Key Capabilities
///
/// * **Multi-Root Aggregation:** Combines arbitrary independent query builders under one root operation.
/// * **Preemptive Metric Aggregation:** Automatically sums cumulative complexity (`Complexity`) across all subqueries and calculates the maximum tree depth (`Depth`).
/// * **Cascading Argument Validation:** Preemptively validates all arguments across all subqueries via [`GqlValidatable`](crate::traits::GqlValidatable).
/// * **Unified Variable Merging:** Consolidates parameters from disparate subqueries into a single unified GraphQL Variables JSON dictionary.
/// * **Zero-Glue Autonomy:** Expands with fully qualified crate paths, requiring zero manual trait imports in downstream caller code.
///
/// # Syntax Breakdown
///
/// ```rust,ignore
/// define_composite_query! {
///     $( #[$struct_meta:meta] )*
///     $vis:vis struct $name:ident {
///         $(
///             $( #[$field_meta:meta] )*
///             $field_fn:ident : $builder_ty:path
///         ),* $(,)?
///     }
/// }
/// ```
///
/// # Example
///
/// ```rust,no_run
/// use moongraphql_builder::prelude::*;
///
/// // 1. Mock field selectors and query builders
/// define_selector! {
///     #[default = "id name"]
///     pub struct ViewerSelector { id: id, name: name }
/// }
/// define_selector! {
///     #[default = "id title"]
///     pub struct ItemSelector { id: id, title: title }
/// }
///
/// define_query_builder! {
///     #[root = viewer]
///     #[selector = ViewerSelector]
///     pub struct ViewerQueryBuilder {}
/// }
/// define_query_builder! {
///     #[root = items]
///     #[selector = ItemSelector]
///     pub struct ItemsQueryBuilder {
///         #[validate(min = 1)]
///         limit: u32 => limit("Int", scalar),
///     }
/// }
///
/// // 2. Generate composite multi-root coordinator
/// define_composite_query! {
///     /// Master coordinator for composite catalog operations.
///     pub struct CatalogCompositeQuery {
///         /// Current authenticated viewer profile.
///         viewer: ViewerQueryBuilder,
///         /// Collection of catalog items.
///         items: ItemsQueryBuilder,
///     }
/// }
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let mut composite = CatalogCompositeQuery::new();
/// composite.viewer(|q| q.select(|f| f.id().name()));
/// composite.items(|q| q.limit(10).select(|f| f.id().title()));
///
/// let (query_doc, variables) = composite.build_payload_parts();
/// println!("Composite Document:\n{}", query_doc);
/// # Ok(())
/// # }
/// ```
#[macro_export]
macro_rules! define_composite_query {
    (
        $( #[$($struct_meta:tt)*] )*
        $vis:vis struct $name:ident {
            $(
                $( #[$($field_meta:tt)*] )*
                $field_fn:ident : $builder_ty:path
            ),* $(,)?
        }
    ) => {
        $( #[$($struct_meta)*] )*
        #[derive(Debug, Default, Clone)]
        $vis struct $name {
            $(
                $( #[$($field_meta)*] )*
                pub(crate) $field_fn: Option<$builder_ty>,
            )*
            /// Explicit complexity budget ceiling override.
            pub(crate) max_complexity_override: Option<u32>,
            /// Explicit tree nesting depth ceiling override.
            pub(crate) max_depth_override: Option<u32>,
            /// Configuration flag enabling or disabling GraphQL variable extraction.
            pub(crate) use_variables: bool,
            /// Optional operation name selector (`operationName`) in payload envelopes.
            pub(crate) operation_name: Option<String>,
            /// Optional manual custom variables JSON dictionary.
            pub(crate) custom_variables: Option<serde_json::Value>,
        }

        impl $name {
            /// Instantiates a new, empty composite GraphQL document coordinator.
            ///
            /// All subquery builders default to `None` until configured via their respective builder methods.
            ///
            /// # Returns
            ///
            /// A default-initialized coordinator instance.
            pub fn new() -> Self {
                Self::default()
            }

            /// Enables or disables automatic variable parametrization across all subqueries.
            ///
            /// # Arguments
            ///
            /// * `enable` (`bool`) — When `true`, all parameters are extracted into `$variable` declarations and JSON dictionaries.
            ///
            /// # Returns
            ///
            /// Mutable reference to `self` for fluent method chaining.
            pub fn use_variables(&mut self, enable: bool) -> &mut Self {
                self.use_variables = enable;
                self
            }

            // Generate fluent configuration methods for each registered subquery operation
            $(
                $( #[$($field_meta)*] )*
                /// Configures this subquery operation via a builder closure.
                ///
                /// # Arguments
                ///
                /// * `builder_fn` (`F`) — Closure receiving the subquery builder instance and returning the configured builder.
                ///
                /// # Returns
                ///
                /// Mutable reference to `self` for fluent method chaining.
                pub fn $field_fn<F>(&mut self, builder_fn: F) -> &mut Self
                where
                    F: FnOnce($builder_ty) -> $builder_ty,
                {
                    let builder = self.$field_fn.take().unwrap_or_default();
                    self.$field_fn = Some(builder_fn(builder));
                    self
                }
            )*

            /// Sets a custom GraphQL operation name (`operationName`) in the JSON payload envelope.
            ///
            /// # Arguments
            ///
            /// * `name` (`impl Into<String>`) — Target operation name identifier.
            ///
            /// # Returns
            ///
            /// Mutable reference to `self` for fluent method chaining.
            pub fn with_operation_name(&mut self, name: impl Into<String>) -> &mut Self {
                self.operation_name = Some(name.into());
                self
            }

            /// Supplies an explicit custom JSON variables dictionary (`variables`).
            ///
            /// # Arguments
            ///
            /// * `variables` ([`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html)) — Custom variables JSON object.
            ///
            /// # Returns
            ///
            /// Mutable reference to `self` for fluent method chaining.
            pub fn with_variables(&mut self, variables: serde_json::Value) -> &mut Self {
                self.custom_variables = Some(variables);
                self
            }

            /// Retrieves the configured operation name (if explicitly set).
            ///
            /// # Returns
            ///
            /// `Some(&str)` containing the operation name, or `None` if omitted.
            pub fn operation_name(&self) -> Option<&str> {
                self.operation_name.as_deref()
            }

            /// Retrieves an immutable reference to the custom variables dictionary (if configured).
            ///
            /// # Returns
            ///
            /// `Some(&Value)` reference, or `None` if absent.
            pub fn variables(&self) -> Option<&serde_json::Value> {
                self.custom_variables.as_ref()
            }

            /// Explicitly overrides the maximum theoretical complexity budget limit for this composite document.
            ///
            /// # Arguments
            ///
            /// * `max` (`u32`) — Maximum allowable cumulative complexity points score.
            ///
            /// # Returns
            ///
            /// Mutable reference to `self` for fluent method chaining.
            pub fn with_max_complexity(&mut self, max: u32) -> &mut Self {
                self.max_complexity_override = Some(max);
                self
            }

            /// Explicitly overrides the maximum theoretical nesting depth ceiling for this composite document.
            ///
            /// # Arguments
            ///
            /// * `max` (`u32`) — Maximum allowable tree depth ceiling.
            ///
            /// # Returns
            ///
            /// Mutable reference to `self` for fluent method chaining.
            pub fn with_max_depth(&mut self, max: u32) -> &mut Self {
                self.max_depth_override = Some(max);
                self
            }

            /// Retrieves the effective complexity budget limit applicable to this composite document.
            ///
            /// Returns the local override if specified, otherwise queries the maximum configured limit
            /// across all included active subqueries.
            ///
            /// # Returns
            ///
            /// `Some(limit)` if configured, or `None` if unconstrained.
            pub fn max_complexity(&self) -> Option<u32> {
                if let Some(max) = self.max_complexity_override {
                    return Some(max);
                }
                $(
                    if let Some(ref b) = self.$field_fn {
                        if let Some(max) = $crate::traits::GqlInspectable::max_complexity(b) {
                            return Some(max);
                        }
                    }
                )*
                None
            }

            /// Retrieves the effective AST nesting depth ceiling applicable to this composite document.
            ///
            /// Returns the local override if specified, otherwise queries the maximum configured depth ceiling
            /// across all included active subqueries.
            ///
            /// # Returns
            ///
            /// `Some(depth)` if configured, or `None` if unconstrained.
            pub fn max_depth(&self) -> Option<u32> {
                if let Some(max) = self.max_depth_override {
                    return Some(max);
                }
                $(
                    if let Some(ref b) = self.$field_fn {
                        if let Some(max) = $crate::traits::GqlInspectable::max_depth(b) {
                            return Some(max);
                        }
                    }
                )*
                None
            }

            /// Calculates the **cumulative complexity score (Complexity)** across all operations included in the document.
            ///
            /// Sums the individual theoretical complexity scores of all configured subqueries.
            ///
            /// # Returns
            ///
            /// Cumulative complexity score as an unsigned 32-bit integer.
            pub fn complexity(&self) -> u32 {
                let mut total = 0;
                $(
                    if let Some(ref b) = self.$field_fn {
                        total += $crate::traits::GqlInspectable::complexity(b);
                    }
                )*
                total
            }

            /// Calculates the **maximum AST nesting depth (Depth)** among all operations included in the document.
            ///
            /// Evaluates the deepest nesting hierarchy found among all configured subqueries.
            ///
            /// # Returns
            ///
            /// Maximum nesting depth as an unsigned 32-bit integer.
            pub fn depth(&self) -> u32 {
                let mut max_depth = 1;
                $(
                    if let Some(ref b) = self.$field_fn {
                        max_depth = max_depth.max($crate::traits::GqlInspectable::depth(b));
                    }
                )*
                max_depth
            }

            /// Extracts composite document components: subqueries, variable declarations, and merged JSON variables.
            ///
            /// Pre-allocates vector and dictionary buffers based on registered field count to eliminate
            /// intermediate re-allocations and heap churn during multi-root aggregation.
            ///
            /// # Returns
            ///
            /// A tuple of:
            /// 1. Vector of compiled subquery body strings.
            /// 2. Consolidated vector of all variable declarations across operations.
            /// 3. Merged [`serde_json::Map`](https://docs.rs/serde_json/latest/serde_json/struct.Map.html) of variable values.
            pub fn build_payload_details(
                &self,
            ) -> (
                Vec<String>,
                Vec<String>,
                serde_json::Map<String, serde_json::Value>,
            ) {
                // Pre-calculate registered subquery count to reserve exact buffer capacities
                let field_count = [$( stringify!($field_fn) ),*].len();

                let mut root_queries = Vec::with_capacity(field_count);
                let mut all_decls = Vec::with_capacity(field_count * 2);
                let mut all_vars = serde_json::Map::with_capacity(field_count * 2);

                $(
                    if let Some(ref b) = self.$field_fn {
                        let (q_part, decls, vars) = $crate::traits::BuildableQuery::_build_query_parts(b, self.use_variables);
                        root_queries.push(q_part);
                        all_decls.extend(decls);
                        all_vars.extend(vars);
                    }
                )*

                if let Some(custom) = &self.custom_variables {
                    if let Some(obj) = custom.as_object() {
                        for (k, v) in obj {
                            all_vars.insert(k.clone(), v.clone());
                        }
                    }
                }

                (root_queries, all_decls, all_vars)
            }

            /// Compiles the final cohesive composite GraphQL document string and its merged `variables` dictionary.
            ///
            /// Assembles the complete document in a single pre-sized buffer, completely bypassing
            /// intermediate `.join()` and `format!()` heap allocations.
            ///
            /// # Returns
            ///
            /// A tuple of:
            /// 1. Final compiled GraphQL document string.
            /// 2. Optional [`serde_json::Value::Object`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html#variant.Object) containing merged variables.
            pub fn build_payload_parts(&self) -> (String, Option<serde_json::Value>) {
                let (root_queries, all_decls, all_vars) = self.build_payload_details();

                let final_query = {
                    let has_decls = self.use_variables && !all_decls.is_empty();

                    // Pre-calculate exact buffer capacity byte-for-byte:
                    // "query () {  }" is 14 bytes, "query {  }" is 10 bytes
                    let prefix_and_wrap_len = if has_decls { 14 } else { 10 };
                    let decls_len: usize = if has_decls {
                        all_decls.iter().map(|d| d.len()).sum::<usize>()
                            + (all_decls.len().saturating_sub(1) * 2) // separator ", "
                    } else {
                        0
                    };
                    let roots_len: usize = root_queries.iter().map(|r| r.len()).sum::<usize>()
                        + root_queries.len().saturating_sub(1); // whitespace separator " "

                    let mut doc = String::with_capacity(prefix_and_wrap_len + decls_len + roots_len);

                    if has_decls {
                        doc.push_str("query (");
                        for (i, decl) in all_decls.iter().enumerate() {
                            if i > 0 {
                                doc.push_str(", ");
                            }
                            doc.push_str(decl);
                        }
                        doc.push_str(") { ");
                    } else {
                        doc.push_str("query { ");
                    }

                    for (i, root) in root_queries.iter().enumerate() {
                        if i > 0 {
                            doc.push(' ');
                        }
                        doc.push_str(root);
                    }

                    doc.push_str(" }");
                    doc
                };

                let final_vars = if all_vars.is_empty() {
                    None
                } else {
                    Some(serde_json::Value::Object(all_vars))
                };

                (final_query, final_vars)
            }

            /// Compiles the final inline GraphQL document string combining all subqueries.
            ///
            /// Inlines all literal argument values directly without variable declarations.
            ///
            /// # Returns
            ///
            /// Formatted inline GraphQL query string.
            pub fn build_query(&self) -> String {
                let (q, _) = self.build_payload_parts();
                q
            }

            /// Internal alias for compiling payload parts.
            #[doc(hidden)]
            pub fn _build_payload_parts(&self) -> (String, Option<serde_json::Value>) {
                self.build_payload_parts()
            }

            /// Internal alias for compiling an inline document string.
            #[doc(hidden)]
            pub fn _build_query(&self) -> String {
                self.build_query()
            }
        }

        // Implement AST metric inspection contract
        impl $crate::traits::GqlInspectable for $name {
            fn complexity(&self) -> u32 {
                self.complexity()
            }

            fn depth(&self) -> u32 {
                self.depth()
            }

            fn max_complexity(&self) -> Option<u32> {
                self.max_complexity()
            }

            fn max_depth(&self) -> Option<u32> {
                self.max_depth()
            }
        }

        // Implement preemptive argument validation contract cascading checks across all active operations
        impl $crate::traits::GqlValidatable for $name {
            fn validate_args(&self) -> $crate::error::Result<()> {
                $(
                    if let Some(ref b) = self.$field_fn {
                        $crate::traits::GqlValidatable::validate_args(b)?;
                    }
                )*
                Ok(())
            }
        }

        // Implement composite GraphQL document trait
        impl $crate::traits::CompositeQueryDocument for $name {
            fn use_variables(&mut self, enable: bool) -> &mut Self {
                self.use_variables(enable)
            }

            fn build_payload_details(
                &self,
            ) -> (
                Vec<String>,
                Vec<String>,
                serde_json::Map<String, serde_json::Value>,
            ) {
                self.build_payload_details()
            }
        }
    };
}
