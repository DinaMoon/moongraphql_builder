# MoonGraphQL Builder 🌙

<div align="center">

[![Crates.io](https://img.shields.io/crates/v/moongraphql_builder.svg?style=flat-square)](https://crates.io/crates/moongraphql_builder)
[![Documentation](https://docs.rs/moongraphql_builder/badge.svg?style=flat-square)](https://docs.rs/moongraphql_builder)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2024%20edition-orange.svg?style=flat-square)](https://www.rust-lang.org)

**English** | [Русский](README.ru.md)

A high-performance, strictly typed GraphQL query and selector construction framework engineered for mission-critical microservices and robust API client libraries.

</div>

---

## 📑 Table of Contents

- [Overview](#-overview)
- [Key Features](#-key-features)
- [Installation](#-installation)
- [Quick Start](#-quick-start)
- [Core Concepts](#-core-concepts)
  - [1. Compile-Time Type-State Selectors (`define_selector!`)](#1-compile-time-type-state-selectors-define_selector)
  - [2. Preemptive AST Metrics & Budgeting (Complexity & Depth)](#2-preemptive-ast-metrics--budgeting-complexity--depth)
  - [3. Declarative Query Builders & Hybrid Validation (`define_query_builder!`)](#3-declarative-query-builders--hybrid-validation-define_query_builder)
  - [4. Dual Compilation Modes (Inline vs. GraphQL Variables)](#4-dual-compilation-modes-inline-vs-graphql-variables)
  - [5. Composite Multi-Root Queries (`define_composite_query!`)](#5-composite-multi-root-queries-define_composite_query)
  - [6. Project Fluent Localization (`i18n`)](#6-project-fluent-localization-i18n)
- [Architecture & Re-exports](#-architecture--re-exports)
- [License](#-license)

---

## 🌟 Overview

Writing raw GraphQL strings in production applications is notoriously prone to subtle bugs: accidental duplicate fields, typo-ridden variable bindings, missing mandatory arguments, and catastrophic HTTP `422 Unprocessable Entity` or `429 Too Many Requests` rejections when queries exceed upstream schema complexity ceilings.

`moongraphql_builder` eliminates these vulnerabilities at their root. Utilizing compile-time Type-State mechanics, declarative macro meta-programming, and Mozilla's Project Fluent, it allows you to assemble mathematically verified, deduplicated, and bounds-checked GraphQL documents entirely in memory before touching the network.

---

## ⚡ Key Features

- 🔒 **Compile-Time Type-State:** Duplicate field selection is physically rejected by the Rust compiler during static analysis using zero-cost generic booleans.
- 📐 **Deterministic Canonical AST:** Fields are automatically deduplicated and alphabetically sorted via `BTreeSet`, guaranteeing predictable query hashes and maximizing HTTP cache hits.
- 📊 **Preemptive Metrics Calculation:** Real-time recursive evaluation of theoretical query **Complexity** and AST nesting **Depth** prevents schema quota violations before dispatch.
- 🛡️ **Hybrid Declarative Validation:** Macro-level validation rules (`#[validate(required)]`, `range`, `min`, `max`, `min_len`, `max_len`, and custom predicates) with context-aware `ArgMeta` diagnostic reporting.
- 🔄 **Dual Compilation Engine:** Compiles into either self-contained inline GraphQL documents or production-ready parameterized GraphQL Variables payloads paired with strongly typed JSON dictionaries.
- 🧩 **Composite Query Coordinators:** Declaratively merge multiple independent subqueries into a single root operation executed over 1 network round-trip (1 RTT).
- 🌍 **Native Project Fluent i18n:** Zero-overhead localized error rendering supporting dual-locale (`en` / `ru`) diagnostics with Task-Local context propagation.

---

## 📦 Installation

Add `moongraphql_builder` to your `Cargo.toml`:

```toml
[dependencies]
moongraphql_builder = "0.1.0"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

---

## 🚀 Quick Start

Here is a minimal standalone example demonstrating selector declaration, query assembly, local metric evaluation, and client-side validation:

```rust
use moongraphql_builder::prelude::*;

// 1. Declare a typed field selector
define_selector! {
    #[default_complexity = 1]
    #[default = "id name"]
    pub struct AnimeSelector {
        #[complexity = 1]
        id: id,
        #[complexity = 1]
        name: name,
        #[complexity = 1]
        russian: russian,
        #[complexity = 1]
        score: score,
    }
}

// 2. Declare a query builder with boundary constraints
define_query_builder! {
    #[root = animes]
    #[selector = AnimeSelector]
    pub struct AnimesQueryBuilder {
        #[validate(required)]
        #[validate(min = 1)]
        page: u32 => page("PositiveInt", scalar),

        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        #[validate(min_len = 2)]
        search: String => search("String", string),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 3. Assemble query with fluent syntax
    let query = AnimesQueryBuilder::new()
        .page(1)
        .limit(10)
        .search("Monogatari")
        .select(|f| f.id().name().russian().score());

    // 4. Preemptive metric verification
    println!("Query Complexity: {}", query.complexity()); // Complexity: 5 (1 root + 4 fields)
    println!("Query Depth:      {}", query.depth());      // Depth: 1

    // 5. Client-side preemptive validation
    let validator = MetricValidator::new(190, 5);
    validator.validate(&query)?;

    // 6. Compile to clean inline GraphQL string
    let raw_query = query._build_query();
    println!("Compiled Document:\n{}", raw_query);

    Ok(())
}
```

---

## 🧠 Core Concepts

### 1. Compile-Time Type-State Selectors (`define_selector!`)

Traditional query builders allow you to call `.id().id().name()` repeatedly, wasting bandwidth or generating redundant AST nodes. `moongraphql_builder` enforces field uniqueness directly within the type system:

```rust
define_selector! {
    #[default_complexity = 5]
    #[default = "originalUrl mainUrl"]
    pub struct PosterSelector {
        #[complexity = 1]
        id: id,
        #[complexity = 1]
        original_url: originalUrl,
        #[complexity = 1]
        main_url: mainUrl,
    }
}
```

- **Closure Type Inference:** In nested closures like `.poster(|p| p.original_url())`, the type of `p` is inferred automatically by the compiler.
- **Identity Fallback:** Calling `.poster(|p| p)` automatically falls back to default fields (`originalUrl mainUrl`).
- **Compile-Time Rejection:** Calling `.id().id()` results in a static compiler error—not a runtime panic.

### 2. Preemptive AST Metrics & Budgeting (Complexity & Depth)

Backends like Apollo Server and GraphQL Ruby enforce strict query complexity points and maximum depth ceilings. Exceeding these limits wastes network bandwidth and returns HTTP 4xx errors.

`moongraphql_builder` calculates cumulative metrics **locally in memory**:

**Total Complexity** = `Root Base Weight` + `∑(Field Weights)` + `∑(Nested Object Weights)`

```rust
let query = AnimesQueryBuilder::new()
    .page(1)
    .with_max_complexity(250) // Local override budget
    .with_max_depth(5)        // Local override depth ceiling
    .select(|f| f.id().name().poster(|p| p.original_url()));

assert_eq!(query.complexity(), 1 + 1 + 1 + 5 + 1); // 9 points
assert_eq!(query.depth(), 2);                      // 2 levels deep
```

### 3. Declarative Query Builders & Hybrid Validation (`define_query_builder!`)

Validate query parameters declaratively before generating payloads:

```rust
fn validate_not_blank(val: &String, meta: &ArgMeta) -> Result<(), String> {
    if val.trim().is_empty() {
        return Err(format!("Argument '{}' cannot be whitespace only", meta.arg_name));
    }
    Ok(())
}

define_query_builder! {
    #[root = animes]
    #[selector = AnimeSelector]
    pub struct AnimesQueryBuilder {
        #[validate(required)]
        page: u32 => page("PositiveInt", scalar),

        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        #[validate(custom = validate_not_blank)]
        search: String => search("String", string),
    }
}
```

Supported validation attributes:
- `#[validate(required)]` — Field must be configured.
- `#[validate(min = N)]` / `#[validate(max = N)]` — Numeric bounds check.
- `#[validate(range = A..=B)]` — Range boundary verification.
- `#[validate(min_len = N)]` / `#[validate(max_len = N)]` — String or slice collection length checks.
- `#[validate(custom = path)]` — Custom predicate receiving `&T` and contextual `&ArgMeta`.

### 4. Dual Compilation Modes (Inline vs. GraphQL Variables)

Depending on your architecture, compile queries into either mode with zero boilerplate:

#### A. Inline Query String
```rust
let inline_doc = query._build_query();
// query { animes(page: 1, limit: 10) { id name } }
```

#### B. Parameterized Document & Variables JSON
```rust
let (body, decls, vars) = query._build_query_parts(true);
let full_doc = format!("query ({}) {{ {} }}", decls.join(", "), body);

// Document: query ($animes_page: PositiveInt, $animes_limit: PositiveInt) { animes(page: $animes_page, limit: $animes_limit) { id name } }
// Variables JSON: {"animes_page": 1, "animes_limit": 10}
```

### 5. Composite Multi-Root Queries (`define_composite_query!`)

Combine multiple disparate subqueries into a single root operation to eliminate HTTP waterfall round-trips:

```rust
define_composite_query! {
    pub struct CatalogCompositeQuery {
        animes: AnimesQueryBuilder,
        mangas: MangasQueryBuilder,
    }
}

let mut composite = CatalogCompositeQuery::new();
composite.animes(|q| q.page(1).limit(5).select(|f| f.id().name()));
composite.mangas(|q| q.page(1).search("Berserk").select(|f| f.id().name()));

let (merged_query, merged_vars) = composite.build_payload_parts();
```

- Automatic variable prefixing prevents naming collisions across operations.
- Aggregated complexity scores evaluate whole-document budget limits in one pass.

### 6. Project Fluent Localization (`i18n`)

All validation errors and metric diagnostics are powered by **Mozilla Project Fluent**:

```rust
use moongraphql_builder::prelude::*;

// Set global locale
set_global_locale("ru");
// Error: "❌ Превышен лимит сложности GraphQL-запроса: рассчитано 250, максимально допустимо 190"

set_global_locale("en");
// Error: "❌ GraphQL query complexity limit exceeded: calculated 250, max allowed 190"
```

External consumer libraries can register domain-specific `.ftl` bundles via `register_resource()`.

---

## 🏗️ Architecture & Re-exports

For seamless interoperability and zero-glue integration, `moongraphql_builder` provides a comprehensive prelude:

```rust
use moongraphql_builder::prelude::*;
```

The prelude cleanly re-exports:
- Declarative macros: `define_selector!`, `define_query_builder!`, `define_composite_query!`, `tr!`.
- Core traits: `BuildableSelector`, `GqlInspectable`, `GqlValidatable`, `BuildableQuery`, `CompositeQueryDocument`.
- Diagnostics & Payloads: `MetricValidator`, `GraphQLPayload`, `GraphQLResponse`, `MoongqlError`, `ArgMeta`.

---

## 📄 License

Licensed under either of:

- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- **MIT license** ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.
