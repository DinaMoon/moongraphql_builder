//! # MoonGraphQL Builder 🌙
//!
//! A high-performance, strictly typed GraphQL query and document generation framework
//! powered by the **Type-State Pattern** (compile-time field deduplication and auto-complete suppression),
//! proactive AST budget metric calculations (`Complexity` and `Depth`), automated GraphQL Variable
//! parametrization, and Project Fluent multi-locale diagnostics.
//!
//! The framework is completely **Transport Agnostic** and can be seamlessly integrated
//! with any asynchronous or blocking HTTP client ([`moonclient`](https://docs.rs/moonclient),
//! [`reqwest`](https://docs.rs/reqwest), [`surf`](https://docs.rs/surf), [`ureq`](https://docs.rs/ureq), etc.).
//!
//! ---
//!
//! ## 🚀 Quick Start
//!
//! Here is a complete, copy-paste-ready example demonstrating typed selector definition,
//! query builder generation with declarative validation, preemptive AST metric checks,
//! and compilation into both inline queries and parameterized GraphQL variables:
//!
//! ```rust
//! use moongraphql_builder::prelude::*;
//!
//! // 1. Declare a Type-State field selector with base complexity
//! define_selector! {
//!     #[default_complexity = 1]
//!     #[default = "id name"]
//!     pub struct UserSelector {
//!         #[complexity = 1]
//!         id: id,
//!         #[complexity = 1]
//!         name: name,
//!         #[complexity = 1]
//!         email: email,
//!     }
//! }
//!
//! // 2. Declare a query builder with boundary constraints
//! define_query_builder! {
//!     #[root = user]
//!     #[selector = UserSelector]
//!     pub struct UserQueryBuilder {
//!         #[validate(required)]
//!         id: u32 => id("ID!", scalar),
//!     }
//! }
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // 3. Assemble query with fluent syntax and compile-time deduplication
//! let query = UserQueryBuilder::new()
//!     .id(42)
//!     .select(|f| f.id().name().email());
//!
//! // 4. Preemptive metric verification
//! assert_eq!(query.complexity(), 1 + 3); // 1 (root) + 3 (fields) = 4
//! assert_eq!(query.depth(), 1);
//!
//! // 5. Preemptive client-side validation
//! let validator = MetricValidator::new(190, 5);
//! validator.validate(&query)?;
//!
//! // 6. Compile to clean inline GraphQL string
//! println!("Compiled Query:\n{}", query._build_query());
//! # Ok(())
//! # }
//! ```
//!
//! ---
//!
//! ## 🏛️ Architectural Pillars
//!
//! - **Compile-Time Field Deduplication:** Utilizes generic `const` booleans and the Type-State
//!   pattern in generated field selectors. Physically removes already-selected fields from IDE code
//!   completion lists and prohibits redundant duplicate selections at compile time without runtime sets.
//! - **Deterministic Canonical AST:** Fields are automatically deduplicated and alphabetically sorted
//!   via [`std::collections::BTreeSet`], guaranteeing reproducible query hashes and optimal HTTP cache hits.
//! - **Proactive AST Complexity & Depth Budgeting:** Calculates theoretical query weight and tree depth
//!   before dispatching requests over the network, protecting applications against remote HTTP `422` and `429`
//!   rejections from servers enforcing strict schema complexity limits.
//! - **Namespaced Variable Parametrization:** Transparently extracts literal values into strongly typed
//!   GraphQL Variables (`$animes_search: String`, `$mangas_limit: PositiveInt`), avoiding variable name
//!   collisions when batching diverse multi-root queries into a single document.
//! - **Multi-Root Composite Documents:** Assemble disparate root queries (`animes`, `mangas`, `users`)
//!   into a single cohesive GraphQL document executed within a single network round-trip time (1 RTT).
//! - **Concurrent Request Batching:** Native HTTP request batching ([`BatchDocumentBuilder`]) with per-query
//!   isolated complexity budgets and parallel asynchronous execution.
//! - **Embedded Localization:** Built-in multi-language diagnostic engine powered by
//!   [`fluent`](https://docs.rs/fluent), supporting dual-locale (`en` / `ru`) validation diagnostics.
//!
//! ---
//!
//! ## 🏗️ Pipeline Execution Flow
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │                 define_selector! & define_query_builder!                     │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │ 1. Static Compile-Time Analysis: Type-State Deduplication (const field: bool)│
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │ 2. Preemptive Metric Evaluation (GqlInspectable: Complexity & Depth)         │
//! │    • Evaluates base weights + field weights locally in memory                │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │ 3. Preemptive Argument Validation (GqlValidatable: min, max, range, custom)  │
//! │    • Localized diagnostic errors rendered via Project Fluent (i18n)          │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │ 4. Dual Compilation Modes (BuildableQuery)                                   │
//! │    ├─► Inline Query:    query { animes(limit: 5) { id name } }               │
//! │    └─► Parameterized:   query ($limit: Int) { ... } + JSON Variables         │
//! └──────────────────────────────────────┬───────────────────────────────────────┘
//!                                        │
//!                                        ▼
//! ┌──────────────────────────────────────────────────────────────────────────────┐
//! │ 5. Network Dispatch (via MoonClient, reqwest, or BatchDocumentBuilder)       │
//! └──────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ---
//!
//! ## 🧩 Feature Flags
//!
//! | Feature | Description | Dependencies | Default |
//! | :--- | :--- | :--- | :---: |
//! | `tokio` | Enables Task-Local locale context storage via [`tokio::task_local`](https://docs.rs/tokio). | `tokio` | **Enabled** |

pub mod batch;
pub mod error;
pub mod i18n;
pub mod macros;
pub mod metrics;
pub mod payload;
pub mod prelude;
pub mod traits;
pub mod validation;

pub use prelude::*;
