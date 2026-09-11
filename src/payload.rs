//! Standardized GraphQL network payload and transport models compliant with the GraphQL over HTTP specification.
//!
//! Provides strongly typed representations for:
//! * Outbound POST JSON request envelopes ([`GraphQLPayload`]).
//! * Universal execution responses ([`GraphQLResponse<T>`]).
//! * Standardized server-side execution diagnostics ([`GraphQLError`]).
//! * Source document error coordinates ([`GraphQLLocation`]).
//!
//! ### Specification Compliance
//!
//! These models strictly adhere to the official [GraphQL over HTTP specification](https://github.com/graphql/graphql-over-http),
//! ensuring seamless interoperability with any standard GraphQL server (Apollo, GraphQL Ruby, Hasura, etc.).
//!
//! ### Quick Example
//!
//! ```rust
//! use moongraphql_builder::payload::{GraphQLPayload, GraphQLResponse};
//! use serde::Deserialize;
//!
//! #[derive(Debug, Deserialize, PartialEq)]
//! struct UserData {
//!     id: u32,
//!     name: String,
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 1. Construct outbound payload
//! let payload = GraphQLPayload {
//!     operation_name: Some("GetUser".to_string()),
//!     query: "query GetUser { user(id: 1) { id name } }".to_string(),
//!     variables: None,
//! };
//!
//! // 2. Deserialize response envelope
//! let raw_json = r#"{"data": {"id": 1, "name": "Dina"}}"#;
//! let response: GraphQLResponse<UserData> = serde_json::from_str(raw_json)?;
//!
//! assert_eq!(response.data, Some(UserData { id: 1, name: "Dina".to_string() }));
//! println!("Parsed GraphQL data successfully!");
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};

/// Line and column position of a syntax or validation error within the GraphQL document source.
///
/// Coordinates follow the 1-indexed convention established by the GraphQL specification.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GraphQLLocation {
    /// Line number within the document text (1-indexed).
    pub line: u32,
    /// Column offset within the line (1-indexed).
    pub column: u32,
}

/// Standardized server-side GraphQL execution error descriptor.
///
/// Follows the official GraphQL specification, detailing human-readable diagnostics,
/// source locations in the document AST, and JSON path coordinates to the offending field.
///
/// # Example
///
/// ```rust
/// use moongraphql_builder::payload::GraphQLError;
///
/// let raw_error = r#"{
///     "message": "Field 'age' is not defined by type 'User'",
///     "locations": [{"line": 1, "column": 15}]
/// }"#;
///
/// let error: GraphQLError = serde_json::from_str(raw_error).unwrap();
/// println!("GraphQL Error: {} at line {:?}", error.message, error.locations);
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct GraphQLError {
    /// Human-readable error message emitted by the GraphQL engine.
    pub message: String,

    /// Optional collection of AST source locations where the error was triggered.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<Vec<GraphQLLocation>>,

    /// Optional JSON path coordinates leading to the field in the data graph that caused the error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<serde_json::Value>>,
}

/// Universal response envelope returned by standard GraphQL endpoints.
///
/// Represents both successful responses, partial executions (where both `data` and `errors` are populated),
/// and complete execution failures.
///
/// # Type Parameters
///
/// * `T` — Deserializable data structure mapping to the requested schema fields.
///
/// # Example: Partial Success Handling
///
/// ```rust
/// use moongraphql_builder::payload::GraphQLResponse;
/// use serde::Deserialize;
///
/// #[derive(Debug, Deserialize)]
/// struct Profile {
///     nickname: String,
/// }
///
/// let response_json = r#"{
///     "data": {"nickname": "DinaMoon"},
///     "errors": [{"message": "Warning: rate limit budget at 90%"}]
/// }"#;
///
/// let response: GraphQLResponse<Profile> = serde_json::from_str(response_json).unwrap();
/// if let Some(data) = response.data {
///     println!("Successfully retrieved data: {}", data.nickname);
/// }
/// if let Some(errors) = response.errors {
///     println!("Encountered execution warnings: {} error(s)", errors.len());
/// }
/// ```
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(bound(deserialize = "T: serde::Deserialize<'de>"))]
pub struct GraphQLResponse<T> {
    /// Deserialized payload data returned by the server (present on full or partial execution success).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Collection of execution or schema validation errors returned by the server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub errors: Option<Vec<GraphQLError>>,
}

/// Standard JSON body payload dispatched to a GraphQL endpoint via HTTP POST.
///
/// Enforces clean serialization by skipping `operationName` and `variables` when [`None`],
/// minimizing wire payload overhead.
///
/// # Example
///
/// ```rust
/// use moongraphql_builder::payload::GraphQLPayload;
///
/// let payload = GraphQLPayload {
///     operation_name: None,
///     query: "query { animes(limit: 5) { id name } }".to_string(),
///     variables: Some(serde_json::json!({ "limit": 5 })),
/// };
///
/// let json_string = serde_json::to_string(&payload).unwrap();
/// assert!(json_string.contains("\"query\":"));
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLPayload {
    /// Optional operation name selector (`operationName`) when multiple operations exist in the document.
    #[serde(rename = "operationName", skip_serializing_if = "Option::is_none")]
    pub operation_name: Option<String>,

    /// Compiled GraphQL document query string.
    pub query: String,

    /// Optional key-value dictionary of GraphQL variables (`variables`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<serde_json::Value>,
}
