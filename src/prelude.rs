//! Canonical crate prelude providing seamless access to public builders, macros, and traits.
//!
//! Designed to be imported via wildcard syntax to bring all core builder primitives,
//! declarative macros, validation traits, and payload structures into scope:
//!
//! ```rust
//! use moongraphql_builder::prelude::*;
//! ```
//!
//! ### What's Included in the Prelude
//!
//! * **Declarative Macros:** [`define_selector!`], [`define_query_builder!`], [`define_composite_query!`], [`tr!`].
//! * **Type-State Traits:** [`BuildableSelector`], [`BuildableQuery`], [`CompositeQueryDocument`], [`GqlInspectable`], [`GqlValidatable`].
//! * **Metrics & Validation:** [`MetricValidator`], [`ArgMeta`].
//! * **Transport Envelopes:** [`GraphQLPayload`], [`GraphQLResponse`], [`GraphQLError`], [`GraphQLLocation`].
//! * **Batch Coordination:** [`BatchDocumentBuilder`].
//! * **Internationalization:** [`set_global_locale`], [`register_resource`].

pub use crate::batch::BatchDocumentBuilder;
pub use crate::define_composite_query;
pub use crate::define_query_builder;
pub use crate::define_selector;
pub use crate::error::{MoongqlError, Result as MoongqlResult};
pub use crate::i18n::{register_resource, set_global_locale};
pub use crate::metrics::MetricValidator;
pub use crate::payload::{GraphQLError, GraphQLLocation, GraphQLPayload, GraphQLResponse};
pub use crate::tr;
pub use crate::traits::{
    BuildableQuery, BuildableSelector, CompositeQueryDocument, GqlInspectable, GqlValidatable,
    IntoSelectorClosure,
};
pub use crate::validation::ArgMeta;
