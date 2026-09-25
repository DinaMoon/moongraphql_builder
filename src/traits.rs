//! Fundamental trait contracts for compiled Type-State GraphQL selectors, query builders, and metrics.
//!
//! This module defines the core abstraction layer of the framework:
//! * [`BuildableSelector`]: AST field selection compilation, deduplication, and recursive complexity/depth evaluation.
//! * [`IntoSelectorClosure`]: Polymorphic selector arguments accepting closures, unit tuples `()`, or optional closures.
//! * [`GqlInspectable`]: Pre-flight inspection of AST complexity weights and tree depth ceilings.
//! * [`GqlValidatable`]: Preemptive client-side argument validation.
//! * [`BuildableQuery`]: Marker trait for fully configured, dispatch-ready GraphQL query builders.
//! * [`CompositeQueryDocument`]: Multi-root document composition within a single HTTP network round-trip.

use crate::error::Result;

/// Core contract implemented by any compiled Type-State field selector.
///
/// Ensures deterministic AST generation, duplicate field prevention, and preemptive
/// calculation of query complexity budgets and tree depth ceilings.
///
/// # Example
///
/// ```rust
/// use moongraphql_builder::traits::BuildableSelector;
///
/// struct UserSelector;
/// impl BuildableSelector for UserSelector {
///     fn build(&self) -> String { "id nickname avatarUrl".to_string() }
///     fn fields_complexity(&self) -> u32 { 3 }
///     fn depth(&self) -> u32 { 1 }
/// }
///
/// let selector = UserSelector;
/// assert_eq!(selector.complexity(), 1 + 3); // 1 (root) + 3 (fields)
/// assert_eq!(selector.depth(), 1);
/// ```
pub trait BuildableSelector {
    /// Default static base complexity score assigned to the root node structure.
    const DEFAULT_COMPLEXITY: u32 = 1;

    /// Compiles the selected fields into a canonical, alphabetically sorted GraphQL field selection string.
    ///
    /// # Returns
    ///
    /// A clean field selection string (e.g. `"id name russian score"`).
    fn build(&self) -> String;

    /// Base theoretical complexity weight of the root selector node itself.
    ///
    /// Defaults to [`Self::DEFAULT_COMPLEXITY`].
    ///
    /// # Returns
    ///
    /// The base complexity weight score as an unsigned 32-bit integer.
    fn root_complexity(&self) -> u32 {
        Self::DEFAULT_COMPLEXITY
    }

    /// Calculates the cumulative complexity score of selected child fields (excluding the root node weight).
    ///
    /// # Returns
    ///
    /// Sum of all child field complexity scores.
    fn fields_complexity(&self) -> u32;

    /// Calculates the full theoretical complexity weight: root node score + sum of child field scores.
    ///
    /// # Returns
    ///
    /// Total cumulative AST complexity weight.
    fn complexity(&self) -> u32 {
        self.root_complexity() + self.fields_complexity()
    }

    /// Calculates the maximum AST nesting depth of the requested field selection graph.
    ///
    /// # Returns
    ///
    /// Maximum nesting depth level (1-indexed).
    fn depth(&self) -> u32;
}

/// Helper trait providing ergonomic polymorphism for passing closures, unit types `()`, or `Option` into nested selectors.
///
/// Powers fluent syntax variations:
/// * Explicit field selection: `.select(|f| f.id().name())`
/// * Default preset forwarding: `.select(|f| f)`
/// * Unit tuple default expansion: `.select(())`
pub trait IntoSelectorClosure<T: BuildableSelector + Default> {
    /// Evaluates the field selection and extracts query components.
    ///
    /// # Returns
    ///
    /// A tuple of `(compiled_fields_string, child_fields_complexity, tree_depth)`.
    fn evaluate_selection(self) -> (String, u32, u32);
}

// 1. Implementation for custom closure expressions: |f| f.id().name()
impl<T, F, S> IntoSelectorClosure<T> for F
where
    T: BuildableSelector + Default,
    F: FnOnce(T) -> S,
    S: BuildableSelector,
{
    fn evaluate_selection(self) -> (String, u32, u32) {
        let initial = T::default();
        let built = self(initial);
        (built.build(), built.fields_complexity(), built.depth())
    }
}

// 2. Implementation for empty unit tuple (): employs the default field selection preset of the target structure
impl<T: BuildableSelector + Default> IntoSelectorClosure<T> for () {
    fn evaluate_selection(self) -> (String, u32, u32) {
        let initial = T::default();
        (
            initial.build(),
            initial.fields_complexity(),
            initial.depth(),
        )
    }
}

// 3. Implementation for optional closures: Some(|f| ...) or None
impl<T, F, S> IntoSelectorClosure<T> for Option<F>
where
    T: BuildableSelector + Default,
    F: FnOnce(T) -> S,
    S: BuildableSelector,
{
    fn evaluate_selection(self) -> (String, u32, u32) {
        match self {
            Some(f) => {
                let initial = T::default();
                let built = f(initial);
                (built.build(), built.fields_complexity(), built.depth())
            }
            None => {
                let initial = T::default();
                (
                    initial.build(),
                    initial.fields_complexity(),
                    initial.depth(),
                )
            }
        }
    }
}

/// Trait implemented by query structures supporting preemptive AST complexity and depth inspection.
pub trait GqlInspectable {
    /// Retrieves the calculated AST complexity score of the query graph.
    ///
    /// # Returns
    ///
    /// The calculated cumulative complexity score.
    fn complexity(&self) -> u32;

    /// Retrieves the calculated AST tree depth of the query graph.
    ///
    /// # Returns
    ///
    /// Maximum nesting depth of the selector tree.
    fn depth(&self) -> u32;

    /// Retrieves the locally overridden complexity budget limit (if explicitly specified).
    ///
    /// # Returns
    ///
    /// `Some(limit)` if configured via `.with_max_complexity(n)`, or `None` to use global validator defaults.
    fn max_complexity(&self) -> Option<u32> {
        None
    }

    /// Retrieves the locally overridden depth budget limit (if explicitly specified).
    ///
    /// # Returns
    ///
    /// `Some(depth)` if configured via `.with_max_depth(n)`, or `None` to use global validator defaults.
    fn max_depth(&self) -> Option<u32> {
        None
    }
}

/// Trait implemented by query builders supporting preemptive client-side parameter validation.
pub trait GqlValidatable {
    /// Validates all configured arguments against declarative constraints and custom rules.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all argument boundaries and custom validation predicates pass.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::MoongqlError::ValidationError`] if any argument
    /// violates boundary limits, format requirements, or custom validation predicates.
    fn validate_args(&self) -> Result<()>;
}

/// Marker trait representing a fully configured GraphQL query ready for payload synthesis.
///
/// If a query builder lacks mandatory arguments tagged with `#[validate(required)]`,
/// the Rust compiler emits a diagnostic compile-time error pointing out the missing argument.
#[diagnostic::on_unimplemented(
    message = "GraphQL query `{Self}` is missing required argument(s)",
    label = "required argument missing",
    note = "call all `#[validate(required)]` arguments on the builder"
)]
pub trait BuildableQuery: GqlInspectable + GqlValidatable {
    /// Compiles the query components into raw body strings, variable declarations, and JSON values.
    ///
    /// # Arguments
    ///
    /// * `use_variables` (`bool`) — If `true`, parameters are extracted into `$variable` definitions.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// 1. Query body string with injected arguments or `$variable` references.
    /// 2. Vector of GraphQL variable declaration signatures (e.g. `"$page: PositiveInt"`).
    /// 3. [`serde_json::Map`](https://docs.rs/serde_json/latest/serde_json/struct.Map.html) of variable values.
    fn _build_query_parts(
        &self,
        use_variables: bool,
    ) -> (
        String,
        Vec<String>,
        serde_json::Map<String, serde_json::Value>,
    );

    /// Compiles a self-contained inline GraphQL query string representation (`query { ... }`).
    ///
    /// Inlines all literal argument values directly into the query document without variable declarations.
    ///
    /// # Returns
    ///
    /// Formatted inline GraphQL query string.
    fn _build_query(&self) -> String {
        let (q, _, _) = self._build_query_parts(false);
        format!("query {{ {} }}", q)
    }
}

/// Trait representing a multi-root composite GraphQL document combining multiple independent operations.
///
/// Enables merging disparate queries (such as `animes`, `mangas`, and `currentUser`) into a single
/// document root executed over 1 network round-trip (1 RTT).
pub trait CompositeQueryDocument: GqlInspectable + GqlValidatable + Default {
    /// Enables or disables automatic variable parametrization (`$var: Type`).
    ///
    /// # Arguments
    ///
    /// * `enable` (`bool`) — When `true`, extracts all subquery arguments into merged GraphQL variables.
    ///
    /// # Returns
    ///
    /// Mutable reference to `Self` for method chaining.
    fn use_variables(&mut self, enable: bool) -> &mut Self;

    /// Extracts internal query payload components across all aggregated operations.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// 1. Vector of compiled subquery body strings.
    /// 2. Consolidated vector of all variable declarations across operations.
    /// 3. Merged [`serde_json::Map`](https://docs.rs/serde_json/latest/serde_json/struct.Map.html) of all variable values.
    fn build_payload_details(
        &self,
    ) -> (
        Vec<String>,
        Vec<String>,
        serde_json::Map<String, serde_json::Value>,
    );

    /// Compiles the final cohesive composite GraphQL document string and its merged `variables` dictionary.
    ///
    /// Merges operation bodies into an outer `query { ... }` or `query ($vars...) { ... }` envelope,
    /// returning the document alongside the optional JSON variables payload.
    ///
    /// # Returns
    ///
    /// A tuple of:
    /// 1. Final compiled GraphQL document string.
    /// 2. Optional [`serde_json::Value::Object`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html#variant.Object) containing merged variables.
    fn build_payload_parts(&self) -> (String, Option<serde_json::Value>) {
        let (root_queries, all_decls, all_vars) = self.build_payload_details();

        let query_body = if root_queries.is_empty() {
            String::new()
        } else {
            root_queries.join(" ")
        };

        let final_query = if !all_decls.is_empty() {
            format!("query ({}) {{ {} }}", all_decls.join(", "), query_body)
        } else {
            format!("query {{ {} }}", query_body)
        };

        let final_vars = if all_vars.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(all_vars))
        };

        (final_query, final_vars)
    }
}
