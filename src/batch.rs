//! Generic HTTP request batching generator and concurrent executor for GraphQL operations.
//!
//! Provides [`BatchDocumentBuilder`] to aggregate multiple independent composite
//! GraphQL queries into an array of payloads (`Vec<GraphQLPayload>`), dispatched
//! either as a single HTTP batch array or executed concurrently in parallel worker tasks.
//!
//! ### Batching vs. Multi-Root Composite Queries
//!
//! | Strategy | Server Complexity Budget | Network RTT | Failure Isolation |
//! | :--- | :--- | :--- | :--- |
//! | **Multi-Root Composite** (`define_composite_query!`) | **Shared:** Sum of all fields must be $\le$ server limit. | 1 RTT | All-or-nothing: one heavy field fails the entire document. |
//! | **HTTP Request Batching** ([`BatchDocumentBuilder`]) | **Isolated:** Each query in the array gets its own full budget! | 1 RTT or Parallel | Isolated: each query evaluates its complexity independently. |
//!
//! ### Execution Architecture
//!
//! ```text
//!                         BatchDocumentBuilder
//!                                   │
//!                  ┌────────────────┴────────────────┐
//!                  ▼                                 ▼
//!            Query 1 (120 pts)                 Query 2 (100 pts)
//!            (Isolated Budget)                 (Isolated Budget)
//!                  │                                 │
//!                  └────────────────┬────────────────┘
//!                                   │
//!                                   ▼
//!               BatchDocumentBuilder::execute_concurrent()
//!                  ┌────────────────┴────────────────┐
//!                  ▼                                 ▼
//!              [Task 1]                          [Task 2]
//!       (Tokio Worker Thread)             (Tokio Worker Thread)
//!                  │                                 │
//!                  └────────────────┬────────────────┘
//!                                   │
//!                                   ▼
//!                  Strict FIFO Ordered Responses: [Res 1, Res 2]
//! ```
//!
//! ### Quick Example
//!
//! ```rust,no_run
//! use moongraphql_builder::prelude::*;
//!
//! # async fn doc_example<Q: CompositeQueryDocument>(mut batch: BatchDocumentBuilder<Q>) -> Result<(), Box<dyn std::error::Error>> {
//! // Concurrently dispatch all queued batch payloads via your HTTP client
//! let responses = batch.execute_concurrent(|payload| async move {
//!     // Transmit payload through moonclient or reqwest:
//!     Ok::<_, String>(payload.query)
//! }).await?;
//!
//! println!("Processed {} batch responses with preserved ordering!", responses.len());
//! # Ok(())
//! # }
//! ```

use crate::error::Result;
use crate::payload::GraphQLPayload;
use crate::traits::{CompositeQueryDocument, GqlInspectable, GqlValidatable};

/// Universal generic builder for HTTP request batching of GraphQL documents.
///
/// Combines a collection of independent composite queries of type `Q` into a batch
/// of payloads (`Vec<GraphQLPayload>`), dispatchable across the wire within a **single network round-trip (1 RTT)**
/// or concurrently across the Tokio thread pool.
///
/// # Type Parameters
///
/// * `Q` — Multi-root composite document structure implementing [`CompositeQueryDocument`].
#[derive(Debug, Default, Clone)]
pub struct BatchDocumentBuilder<Q: CompositeQueryDocument> {
    /// Internal queue of registered composite queries.
    queries: Vec<Q>,
    /// Optional explicit override for maximum allowed complexity ceiling.
    max_complexity_override: Option<u32>,
    /// Optional explicit override for maximum allowed tree depth ceiling.
    max_depth_override: Option<u32>,
    /// Configuration flag enabling or disabling GraphQL variable extraction.
    use_variables: bool,
}

impl<Q: CompositeQueryDocument> BatchDocumentBuilder<Q> {
    /// Instantiates a new, empty batch document builder.
    ///
    /// # Returns
    ///
    /// A clean [`BatchDocumentBuilder<Q>`] instance.
    ///
    /// # Example
    ///
    /// ```rust
    /// use moongraphql_builder::batch::BatchDocumentBuilder;
    /// use moongraphql_builder::traits::CompositeQueryDocument;
    ///
    /// # fn doc_example<Q: CompositeQueryDocument>() {
    /// let batch: BatchDocumentBuilder<Q> = BatchDocumentBuilder::new();
    /// assert_eq!(batch.len(), 0);
    /// # }
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// Enables or disables automatic variable parametrization (`$var: Type`) across all queries in the batch.
    ///
    /// # Arguments
    ///
    /// * `enable` (`bool`) — When `true`, all registered queries extract arguments into JSON variables.
    ///
    /// # Returns
    ///
    /// Mutable reference to `self` for fluent chaining.
    pub fn use_variables(&mut self, enable: bool) -> &mut Self {
        self.use_variables = enable;
        self
    }

    /// Appends an independent composite GraphQL query to the batch via a configuration closure.
    ///
    /// # Arguments
    ///
    /// * `query_fn` (`F`) — Closure receiving a mutable reference to a fresh composite query builder.
    ///
    /// # Returns
    ///
    /// Mutable reference to `self` for fluent chaining.
    ///
    /// # Example
    ///
    /// ```rust
    /// use moongraphql_builder::batch::BatchDocumentBuilder;
    /// use moongraphql_builder::traits::CompositeQueryDocument;
    ///
    /// # fn doc_example<Q: CompositeQueryDocument>(mut batch: BatchDocumentBuilder<Q>) {
    /// batch.query(|q| {
    ///     // Configure subqueries on q...
    ///     q
    /// });
    /// assert_eq!(batch.len(), 1);
    /// # }
    /// ```
    pub fn query<F>(&mut self, query_fn: F) -> &mut Self
    where
        F: FnOnce(&mut Q) -> &mut Q,
    {
        let mut q_builder = Q::default();
        if self.use_variables {
            q_builder.use_variables(true);
        }
        query_fn(&mut q_builder);
        self.queries.push(q_builder);
        self
    }

    /// Explicitly overrides the maximum theoretical complexity budget for queries in this batch.
    ///
    /// # Arguments
    ///
    /// * `max` (`u32`) — New maximum allowable complexity points ceiling.
    ///
    /// # Returns
    ///
    /// Mutable reference to `self` for fluent chaining.
    pub fn with_max_complexity(&mut self, max: u32) -> &mut Self {
        self.max_complexity_override = Some(max);
        self
    }

    /// Explicitly overrides the maximum theoretical nesting depth ceiling for queries in this batch.
    ///
    /// # Arguments
    ///
    /// * `max` (`u32`) — New maximum allowable tree depth ceiling.
    ///
    /// # Returns
    ///
    /// Mutable reference to `self` for fluent chaining.
    pub fn with_max_depth(&mut self, max: u32) -> &mut Self {
        self.max_depth_override = Some(max);
        self
    }

    /// Retrieves the effective complexity limit applicable to this batch.
    ///
    /// Returns the local override if set, otherwise queries the maximum configured
    /// limit among individual registered operations.
    ///
    /// # Returns
    ///
    /// `Some(limit)` if a threshold is specified, or `None` if completely unconstrained.
    pub fn max_complexity(&self) -> Option<u32> {
        if let Some(max) = self.max_complexity_override {
            return Some(max);
        }
        self.queries.iter().find_map(|q| q.max_complexity())
    }

    /// Retrieves the effective AST nesting depth ceiling applicable to this batch.
    ///
    /// Returns the local override if set, otherwise queries the maximum depth limit
    /// among individual registered operations.
    ///
    /// # Returns
    ///
    /// `Some(depth)` if a ceiling is specified, or `None` if unconstrained.
    pub fn max_depth(&self) -> Option<u32> {
        if let Some(max) = self.max_depth_override {
            return Some(max);
        }
        self.queries.iter().find_map(|q| q.max_depth())
    }

    /// Calculates the maximum theoretical complexity score among all queries in the batch.
    ///
    /// In request batching, queries are evaluated independently by the server, meaning
    /// the governing metric for pre-flight verification is the **maximum single query weight**.
    ///
    /// # Returns
    ///
    /// The highest complexity score found in the batch, or `0` if empty.
    pub fn complexity(&self) -> u32 {
        self.queries
            .iter()
            .map(|q| q.complexity())
            .max()
            .unwrap_or(0)
    }

    /// Calculates the maximum AST nesting depth among all queries in the batch.
    ///
    /// # Returns
    ///
    /// The deepest nesting level found in the batch, or `1` if empty.
    pub fn depth(&self) -> u32 {
        self.queries.iter().map(|q| q.depth()).max().unwrap_or(1)
    }

    /// Returns the total number of composite queries currently queued in the batch.
    ///
    /// # Returns
    ///
    /// Number of registered operations as a [`usize`].
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// Checks if the batch contains zero queued queries.
    ///
    /// # Returns
    ///
    /// `true` if empty, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }

    /// Compiles all registered queries into a collection of independent [`GraphQLPayload`] items.
    ///
    /// # Returns
    ///
    /// Vector of [`GraphQLPayload`] instances ready for wire transmission or batch serialization.
    pub fn build_payloads(&self) -> Vec<GraphQLPayload> {
        self.queries
            .iter()
            .map(|q| {
                let (query, variables) = q.build_payload_parts();
                GraphQLPayload {
                    operation_name: None,
                    query,
                    variables,
                }
            })
            .collect()
    }

    /// Alias for building the collection of batch payloads.
    #[doc(hidden)]
    pub fn _build_payloads(&self) -> Vec<GraphQLPayload> {
        self.build_payloads()
    }

    /// Concurrently dispatches all batch queries in parallel, guaranteeing strict FIFO response ordering.
    ///
    /// Accepts an asynchronous `executor` closure that transmits a single [`GraphQLPayload`]
    /// via an arbitrary HTTP client (`moonclient`, `reqwest`, etc.).
    ///
    /// # Key Advantages
    ///
    /// * **Server-Agnostic:** Works regardless of whether the remote server natively supports Apollo batching.
    /// * **Budget Isolation:** Each query gets its own independent complexity and depth quotas.
    /// * **Parallel Execution:** Dispatches all network transactions concurrently via [`futures::future::join_all`](https://docs.rs/futures/latest/futures/future/fn.join_all.html).
    /// * **Deterministic Ordering:** Preserves strict FIFO response ordering matching registration order.
    ///
    /// # Arguments
    ///
    /// * `executor` (`F`) — Asynchronous closure transforming a [`GraphQLPayload`] into a dispatch future.
    ///
    /// # Returns
    ///
    /// Returns `Ok(Vec<T>)` containing all response models ordered identically to the queries registration sequence.
    ///
    /// # Errors
    ///
    /// Fails fast with `Err(E)` if any individual execution task produces an error.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use moongraphql_builder::prelude::*;
    ///
    /// # async fn doc_example<Q: CompositeQueryDocument>(batch: BatchDocumentBuilder<Q>) -> Result<(), Box<dyn std::error::Error>> {
    /// let responses = batch.execute_concurrent(|payload| async move {
    ///     // Execute through your transport pipeline:
    ///     Ok::<_, String>(format!("Result for query length: {}", payload.query.len()))
    /// }).await?;
    ///
    /// println!("All parallel requests completed: {:?}", responses);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn execute_concurrent<F, Fut, T, E>(
        &self,
        executor: F,
    ) -> std::result::Result<Vec<T>, E>
    where
        F: Fn(GraphQLPayload) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<T, E>>,
    {
        let payloads = self.build_payloads();
        let mut tasks = Vec::with_capacity(payloads.len());

        for payload in payloads {
            tasks.push(executor(payload));
        }

        let results = futures::future::join_all(tasks).await;
        let mut responses = Vec::with_capacity(results.len());

        for res in results {
            responses.push(res?);
        }

        Ok(responses)
    }
}

// =============================================================================
// 📊 TRAIT IMPLEMENTATIONS
// =============================================================================

impl<Q: CompositeQueryDocument> GqlInspectable for BatchDocumentBuilder<Q> {
    /// Returns the maximum theoretical complexity score found across all queries in this batch.
    fn complexity(&self) -> u32 {
        self.complexity()
    }

    /// Returns the deepest nesting depth found across all queries in this batch.
    fn depth(&self) -> u32 {
        self.depth()
    }

    /// Returns the effective complexity ceiling applicable to this batch.
    fn max_complexity(&self) -> Option<u32> {
        self.max_complexity()
    }

    /// Returns the effective depth ceiling applicable to this batch.
    fn max_depth(&self) -> Option<u32> {
        self.max_depth()
    }
}

impl<Q: CompositeQueryDocument> GqlValidatable for BatchDocumentBuilder<Q> {
    /// Cascades preemptive argument validation across every registered query in the batch.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if all queries pass validation.
    ///
    /// # Errors
    ///
    /// Fails fast with [`crate::error::MoongqlError::ValidationError`] if any argument in any query violates constraints.
    fn validate_args(&self) -> Result<()> {
        for q in &self.queries {
            q.validate_args()?;
        }
        Ok(())
    }
}
