//! Domain error models and result types for the MoonGraphQL Builder framework.
//!
//! Encapsulates schema budget violations (AST query complexity and tree nesting depth)
//! alongside preemptive client-side parameter validation errors, backed by
//! Project Fluent multi-locale string interpolation.
//!
//! ### Error Philosophy: Fail Fast & Preemptively
//!
//! Rather than allowing invalid arguments or overly complex queries to reach the remote
//! GraphQL server (only to fail with expensive HTTP `422` or `429` errors), [`MoongqlError`]
//! detects schema constraint violations **locally in memory**, saving bandwidth and rate-limiting quotas.
//!
//! ### Quick Example
//!
//! ```rust
//! use moongraphql_builder::prelude::*;
//!
//! fn handle_graphql_error(err: MoongqlError) {
//!     match err {
//!         MoongqlError::ComplexityExceeded { calculated, limit } => {
//!             eprintln!("Query too expensive! Cost: {} (Max allowed: {})", calculated, limit);
//!         }
//!         MoongqlError::DepthExceeded { calculated, limit } => {
//!             eprintln!("Query too deeply nested! Depth: {} (Max allowed: {})", calculated, limit);
//!         }
//!         MoongqlError::ValidationError(details) => {
//!             eprintln!("Preemptive argument validation failed: {}", details);
//!         }
//!     }
//! }
//! ```

use thiserror::Error;

/// Primary error type for the `moongraphql_builder` framework.
///
/// Provides automated, localized diagnostic descriptions resolved dynamically via Project Fluent.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum MoongqlError {
    /// GraphQL query complexity score exceeded the allowable budget threshold.
    ///
    /// Triggered when the cumulative AST weight (root node + selected fields + nested objects)
    /// surpasses the configured ceiling.
    #[error("{}", crate::tr!("err-gql-complexity-exceeded", "calculated" => calculated, "limit" => limit))]
    ComplexityExceeded {
        /// Calculated theoretical AST complexity score.
        calculated: u32,
        /// Maximum allowable complexity limit configured for the query or validator.
        limit: u32,
    },

    /// GraphQL query AST nesting depth exceeded the allowable budget threshold.
    ///
    /// Prevents cyclic relationship recursion attacks by enforcing a strict hierarchy ceiling.
    #[error("{}", crate::tr!("err-gql-depth-exceeded", "calculated" => calculated, "limit" => limit))]
    DepthExceeded {
        /// Calculated maximum nesting depth of the selector subtree.
        calculated: u32,
        /// Maximum allowable depth ceiling configured for the query or validator.
        limit: u32,
    },

    /// Preemptive client-side query argument validation failure.
    ///
    /// Triggered when an argument violates declarative constraints (`min`, `max`, `range`,
    /// `min_len`, `max_len`, `required`, or custom predicates) defined via `#[validate(...)]`.
    #[error("{}", crate::tr!("err-gql-validation", "error" => .0))]
    ValidationError(String),
}

/// Ergonomic type alias for [`std::result::Result`] specialized over [`MoongqlError`].
///
/// Used throughout builder validation routines, metric evaluators, and payload compilers.
///
/// # Example
///
/// ```rust
/// use moongraphql_builder::error::{MoongqlError, Result};
///
/// fn verify_complexity(calculated: u32, max_limit: u32) -> Result<()> {
///     if calculated > max_limit {
///         return Err(MoongqlError::ComplexityExceeded {
///             calculated,
///             limit: max_limit,
///         });
///     }
///     Ok(())
/// }
/// ```
pub type Result<T> = std::result::Result<T, MoongqlError>;
