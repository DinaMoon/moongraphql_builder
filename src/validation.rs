//! Context models and metadata descriptors for hybrid client-side GraphQL query validation.
//!
//! Provides diagnostic metadata passed into custom validation predicates, enabling localized
//! reporting of schema constraint violations prior to outbound network dispatch.
//!
//! ### Custom Predicate Integration
//!
//! When defining a query builder via [`define_query_builder!`](crate::define_query_builder),
//! custom validation rules tagged with `#[validate(custom = path)]` receive a reference to the
//! argument value and an immutable [`ArgMeta`] descriptor.
//!
//! ### Quick Example
//!
//! ```rust
//! use moongraphql_builder::validation::ArgMeta;
//!
//! /// Custom validator ensuring a string search term is not blank
//! fn validate_non_blank(value: &String, meta: &ArgMeta) -> Result<(), String> {
//!     if value.trim().is_empty() {
//!         return Err(format!(
//!             "Argument '{}' in query '{}' cannot consist solely of whitespace",
//!             meta.arg_name, meta.root_query
//!         ));
//!     }
//!     Ok(())
//! }
//! ```

/// Metadata descriptor for a validated argument passed into custom validation predicates.
///
/// Carries source code identifiers, remote GraphQL schema parameter names, and the root query context.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArgMeta {
    /// Argument method identifier in Rust source code (e.g. `"limit"` or `"search"`).
    pub arg_name: &'static str,

    /// Argument field identifier defined in the remote GraphQL schema (e.g. `"limit"` or `"excludeIds"`).
    pub gql_arg_name: &'static str,

    /// Root query identifier associated with this argument (e.g. `"animes"` or `"users"`).
    pub root_query: &'static str,
}

impl ArgMeta {
    /// Instantiates a new argument metadata descriptor.
    ///
    /// # Arguments
    ///
    /// * `arg_name` (`&'static str`) — Rust field/method identifier.
    /// * `gql_arg_name` (`&'static str`) — Remote GraphQL parameter name.
    /// * `root_query` (`&'static str`) — Root query operation name.
    ///
    /// # Returns
    ///
    /// A new [`ArgMeta`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use moongraphql_builder::validation::ArgMeta;
    ///
    /// let meta = ArgMeta {
    ///     arg_name: "search",
    ///     gql_arg_name: "search",
    ///     root_query: "animes",
    /// };
    /// println!("Validating argument: '{}' for query '{}'", meta.arg_name, meta.root_query);
    /// ```
    pub const fn new(
        arg_name: &'static str,
        gql_arg_name: &'static str,
        root_query: &'static str,
    ) -> Self {
        Self {
            arg_name,
            gql_arg_name,
            root_query,
        }
    }
}
