//! Pre-flight metric validation engine for AST tree complexity and selector depth.
//!
//! Provides [`MetricValidator`] to enforce configurable safety boundaries on query weight
//! and nesting levels prior to outbound network dispatch, protecting client applications
//! from remote HTTP `422 Unprocessable Entity` and `429 Too Many Requests` rejections.
//!
//! ### Metric Enforcement Pipeline
//!
//! ```text
//!                      MetricValidator::validate(&query)
//!                                     │
//!                                     ▼
//!             ┌───────────────────────────────────────────────┐
//!             │ 1. GqlValidatable::validate_args()            │
//!             │    (Preemptively checks min, max, regex, etc.)│
//!             └───────────────────────┬───────────────────────┘
//!                                     │
//!                                     ▼
//!             ┌───────────────────────────────────────────────┐
//!             │ 2. GqlInspectable::complexity()               │
//!             │    (Root cost + Field costs <= max_complexity)│
//!             └───────────────────────┬───────────────────────┘
//!                                     │
//!                                     ▼
//!             ┌───────────────────────────────────────────────┐
//!             │ 3. GqlInspectable::depth()                    │
//!             │    (AST nesting depth <= max_depth)           │
//!             └───────────────────────────────────────────────┘
//! ```
//!
//! ### Quick Example
//!
//! ```rust
//! use moongraphql_builder::metrics::MetricValidator;
//! use moongraphql_builder::error::MoongqlError;
//! use moongraphql_builder::traits::{GqlInspectable, GqlValidatable};
//!
//! // Mock structure implementing inspectable and validatable contracts
//! struct MockQuery;
//! impl GqlValidatable for MockQuery {
//!     fn validate_args(&self) -> Result<(), MoongqlError> { Ok(()) }
//! }
//! impl GqlInspectable for MockQuery {
//!     fn complexity(&self) -> u32 { 15 }
//!     fn depth(&self) -> u32 { 2 }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Configure budget: max complexity 20, max depth 3
//! let validator = MetricValidator::new(20, 3);
//! validator.validate(&MockQuery)?;
//! println!("Query passed preemptive metric validation!");
//! # Ok(())
//! # }
//! ```

use crate::error::{MoongqlError, Result};
use crate::traits::{GqlInspectable, GqlValidatable};

/// Preemptive metric and argument validator for any GraphQL query builder or composite document.
///
/// Evaluates theoretical AST complexity weights and hierarchy depth ceilings against
/// configured thresholds before network dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MetricValidator {
    /// Default global budget ceiling for query AST theoretical complexity.
    default_max_complexity: u32,
    /// Default global budget ceiling for maximum selector nesting depth.
    default_max_depth: u32,
}

impl MetricValidator {
    /// Instantiates a new metric validator configured with baseline default limits.
    ///
    /// # Arguments
    ///
    /// * `default_max_complexity` (`u32`) — Default ceiling for cumulative AST complexity score.
    /// * `default_max_depth` (`u32`) — Default ceiling for maximum selector nesting levels.
    ///
    /// # Returns
    ///
    /// A configured [`MetricValidator`] ready for query evaluation.
    ///
    /// # Example
    ///
    /// ```rust
    /// use moongraphql_builder::metrics::MetricValidator;
    ///
    /// // Standard ceiling: 190 complexity points, depth limit of 5
    /// let validator = MetricValidator::new(190, 5);
    /// println!("Validator configured with limits: {:?}", validator);
    /// ```
    pub const fn new(default_max_complexity: u32, default_max_depth: u32) -> Self {
        Self {
            default_max_complexity,
            default_max_depth,
        }
    }

    /// Executes comprehensive pre-flight validation on the target GraphQL query.
    ///
    /// # Validation Lifecycle Steps
    ///
    /// 1. **Argument Validation:** Calls [`GqlValidatable::validate_args`]. Fails fast if any declarative rule is violated.
    /// 2. **Complexity Enforcement:** Evaluates [`GqlInspectable::complexity`]. Uses the query's local override limit if configured, otherwise falls back to `default_max_complexity`.
    /// 3. **Depth Ceiling Enforcement:** Evaluates [`GqlInspectable::depth`]. Uses local depth overrides if present, otherwise checks against `default_max_depth`.
    ///
    /// # Arguments
    ///
    /// * `target` (`&(impl `[`GqlInspectable`]` + `[`GqlValidatable`]`)`) — Reference to any query builder or composite document.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if arguments are valid and all AST metrics remain within allowed boundaries.
    ///
    /// # Errors
    ///
    /// * [`MoongqlError::ValidationError`] — If argument constraints (`min`, `range`, custom validators) are breached.
    /// * [`MoongqlError::ComplexityExceeded`] — If cumulative AST weight exceeds the complexity budget.
    /// * [`MoongqlError::DepthExceeded`] — If selector nesting exceeds the depth ceiling.
    ///
    /// # Example
    ///
    /// ```rust
    /// use moongraphql_builder::prelude::*;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let validator = MetricValidator::new(100, 3);
    ///
    /// // Query builders generated via define_query_builder! implement both traits
    /// // validator.validate(&query)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn validate(&self, target: &(impl GqlInspectable + GqlValidatable)) -> Result<()> {
        // 1. Preemptively validate argument values and ranges
        target.validate_args()?;

        // 2. Enforce AST complexity budget
        let max_complexity = target
            .max_complexity()
            .unwrap_or(self.default_max_complexity);
        let calculated_complexity = target.complexity();

        if calculated_complexity > max_complexity {
            return Err(MoongqlError::ComplexityExceeded {
                calculated: calculated_complexity,
                limit: max_complexity,
            });
        }

        // 3. Enforce graph tree depth budget
        let max_depth = target.max_depth().unwrap_or(self.default_max_depth);
        let calculated_depth = target.depth();

        if calculated_depth > max_depth {
            return Err(MoongqlError::DepthExceeded {
                calculated: calculated_depth,
                limit: max_depth,
            });
        }

        Ok(())
    }
}
