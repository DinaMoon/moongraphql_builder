//! Declarative macro generators for Type-State GraphQL selectors, query builders, and composite coordinators.
//!
//! Metaprogramming is the primary engine of `moongraphql_builder`. It eliminates repetitive boilerplate,
//! generates compile-time state machines for duplicate field prevention, and wires together preemptive
//! AST metric evaluation without runtime reflection.
//!
//! ### Macro Ecosystem Overview
//!
//! | Macro | Subsystem | Responsibility |
//! | :--- | :--- | :--- |
//! | [`define_selector!`](crate::define_selector) | `selector.rs` | Generates Type-State field selectors with compile-time deduplication and closure type inference. |
//! | [`define_query_builder!`](crate::define_query_builder) | `builder.rs` | Generates strongly typed query builders with declarative argument validation (`#[validate]`). |
//! | [`define_composite_query!`](crate::define_composite_query) | `composite.rs` | Generates multi-root coordinators combining disparate operations into a single network round-trip (1 RTT). |

pub mod builder;
pub mod composite;
pub mod selector;
