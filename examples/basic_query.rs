//! =============================================================================
//! 🌙 MoonGraphQL Builder — Reference Showcase & Lifecycle Example
//! =============================================================================
//!
//! This standalone example demonstrates the complete end-to-end lifecycle of the
//! [`moongraphql_builder`] framework:
//!
//! 1. **Type-State Selectors ([`define_selector!`]):** Declarative field selection with
//!    compile-time duplicate prevention and automatic closure type inference.
//! 2. **Query Assembly ([`define_query_builder!`]):** Generating a type-safe fluent builder
//!    with hybrid argument validation (`min`, `range`, `min_len`, and custom predicates).
//! 3. **Pre-flight AST Metrics:** Preemptive local calculation of query theoretical weight
//!    ([`GqlInspectable::complexity`]) and nesting hierarchy ([`GqlInspectable::depth`]).
//! 4. **Dual Compilation Engine:** Compiling queries into either self-contained inline strings
//!    or production-ready parameterized GraphQL Variables payloads paired with JSON maps.
//! 5. **Preemptive Metric Validation ([`MetricValidator`]):** Enforcing schema budgets locally
//!    before touching the network to eliminate remote HTTP `422` or `429` rejections.

use moongraphql_builder::prelude::*;

// Bind high-performance mimalloc memory allocator to accelerate Tokio worker threads
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// =============================================================================
// 🖼️ 1. NESTED OBJECT SELECTOR (POSTER)
// =============================================================================

define_selector! {
    /// Field selector for poster images (`Poster`) in GraphQL queries.
    ///
    /// Provides access to image assets at various resolutions.
    #[default_complexity = 5]
    #[default = "originalUrl mainUrl"]
    pub struct PosterSelector {
        /// Unique poster identifier (`id`).
        #[complexity = 1]
        id: id,

        /// URL pointing to the original image in maximum resolution (`originalUrl`).
        #[complexity = 1]
        original_url: originalUrl,

        /// Standard resolution display URL (`mainUrl`).
        #[complexity = 1]
        main_url: mainUrl,
    }
}

// =============================================================================
// 📺 2. ROOT ENTITY SELECTOR (ANIME)
// =============================================================================

define_selector! {
    /// Field selector for anime entities (`Anime`) in GraphQL queries.
    ///
    /// Enforces strict compile-time Type-State duplicate field prevention.
    #[default_complexity = 1]
    #[default = "id name russian"]
    pub struct AnimeSelector {
        /// Unique numeric title identifier (`id`).
        #[complexity = 1]
        id: id,

        /// Original title in romaji or English (`name`).
        #[complexity = 1]
        name: name,

        /// Officially localized title name in Russian (`russian`).
        #[complexity = 1]
        russian: russian,

        /// Average user score on a 10-point scale (`score`).
        #[complexity = 1]
        score: score,

        /// Nested poster image object (`poster`).
        #[complexity = 5]
        poster(PosterSelector): poster,
    }
}

// =============================================================================
// 🚀 3. CUSTOM VALIDATOR & QUERY BUILDER (ANIMES)
// =============================================================================

/// Custom validation predicate verifying that a textual search query is not blank.
///
/// Demonstrates utilizing contextual [`ArgMeta`] metadata to generate informative,
/// user-friendly diagnostic error messages.
///
/// # Arguments
///
/// * `val` ([`&String`]) — Candidate search string passed to the builder.
/// * `meta` ([`&ArgMeta`]) — Metadata descriptor carrying parameter and root query names.
///
/// # Returns
///
/// Returns `Ok(())` if the string contains at least one non-whitespace character.
///
/// # Errors
///
/// Emits an informative error string if `val` consists entirely of whitespace characters.
///
/// # Example
///
/// ```rust
/// use moongraphql_builder::validation::ArgMeta;
/// # fn validate_not_blank(val: &String, meta: &ArgMeta) -> Result<(), String> {
/// #     if val.trim().is_empty() { return Err(format!("Argument '{}' invalid", meta.arg_name)); } Ok(())
/// # }
///
/// let meta = ArgMeta::new("search", "search", "animes");
/// assert!(validate_not_blank(&"Bakemonogatari".to_string(), &meta).is_ok());
/// assert!(validate_not_blank(&"   ".to_string(), &meta).is_err());
/// ```
fn validate_not_blank(val: &String, meta: &ArgMeta) -> Result<(), String> {
    if val.trim().is_empty() {
        return Err(format!(
            "Argument '{}' in query '{}' cannot consist solely of whitespace",
            meta.arg_name, meta.root_query
        ));
    }
    Ok(())
}

define_query_builder! {
    /// Argument builder and generator for the anime list query (`Query.animes`).
    ///
    /// Encapsulates declarative validation rules, manages GraphQL variables serialization,
    /// and automatically computes cumulative graph metrics.
    #[root = animes]
    #[selector = AnimeSelector]
    pub struct AnimesQueryBuilder {
        /// Requested page index (minimum 1).
        #[validate(min = 1)]
        page: u32 => page("PositiveInt", scalar),

        /// Items count limit per page (bounded between 1 and 50).
        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        /// Search query: bounded length (2..=100) + custom non-blank validator.
        #[validate(min_len = 2)]
        #[validate(max_len = 100)]
        #[validate(custom = validate_not_blank)]
        search: String => search("String", string),

        /// Exact identifier filtering list (maximum 5 elements).
        #[validate(max_len = 5)]
        ids: Vec<String> => ids("[ID!]", list),
    }
}

// =============================================================================
// 🧪 4. ENTRY POINT & FEATURE DEMO
// =============================================================================

/// Entry point executing the demonstration pipeline across all 5 framework stages.
fn main() {
    println!("===============================================================================");
    println!("🌙 MoonGraphQL Builder — Showcase & Verification Harness");
    println!("===============================================================================\n");

    // -------------------------------------------------------------------------
    // 1. Fluent Query Assembly
    // -------------------------------------------------------------------------
    let query = AnimesQueryBuilder::new()
        .page(1)
        .search("Bakemonogatari")
        .limit(5)
        .ids(["1", "2", "3"])
        .select(|f| f.id().name().russian().score().poster(|p| p.original_url()));

    // -------------------------------------------------------------------------
    // 2. Preemptive AST Metric Calculation (Complexity & Depth)
    // -------------------------------------------------------------------------
    println!("📊 1. Computed AST Metrics:");
    println!("   • Complexity: {}", query.complexity());
    println!("   • Depth:      {}", query.depth());

    // Mathematical verification:
    // 1 (root animes) + 1 (id) + 1 (name) + 1 (russian) + 1 (score) + 5 (poster node) + 1 (originalUrl) = 11
    assert_eq!(query.complexity(), 1 + 1 + 1 + 1 + 1 + 5 + 1);
    assert_eq!(query.depth(), 2);

    // -------------------------------------------------------------------------
    // 3. Compilation into Clean Inline GraphQL Document
    // -------------------------------------------------------------------------
    println!("\n📝 2. Compiled Inline Query:");
    println!("   {}", query._build_query());

    // -------------------------------------------------------------------------
    // 4. Compilation into Parameterized GraphQL Variables Document
    // -------------------------------------------------------------------------
    let (query_str, decls, vars) = query._build_query_parts(true);
    let full_query = format!("query ({}) {{ {} }}", decls.join(", "), query_str);

    println!("\n📦 3. Parameterized Query with GraphQL Variables:");
    println!("   [Document]:\n   {}", full_query);
    println!(
        "\n   [Variables JSON]:\n{}",
        serde_json::to_string_pretty(&vars).expect("Failed to serialize variables JSON")
    );

    // -------------------------------------------------------------------------
    // 5. Preemptive Boundary & Argument Validation Prior to Network Dispatch
    // -------------------------------------------------------------------------
    println!("\n🛡️ 4. Comprehensive Preemptive Validation (Arguments + Metrics):");
    let validator = MetricValidator::new(190, 5);

    match validator.validate(&query) {
        Ok(_) => {
            println!("   ✅ Status: Query is fully valid and safe for network dispatch!")
        }
        Err(err) => eprintln!("   ❌ Error: {}", err),
    }

    println!("\n===============================================================================");
}
