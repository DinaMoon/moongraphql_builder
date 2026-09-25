//! Comprehensive integration test suite for the [`moongraphql_builder`] crate.
//!
//! Validates:
//! * Type-State field deduplication and deterministic alphabetical sorting in [`std::collections::BTreeSet`].
//! * Nested selector AST complexity and tree depth calculations.
//! * Inline query compilation and GraphQL Variables JSON generation.
//! * Preemptive metric budget enforcement via [`MetricValidator`].
//! * Multi-locale Project Fluent error message rendering (`en` / `ru`).
//! * Declarative argument boundary validation (`min`, `max`, `range`, `min_len`, `max_len`, `custom`).

use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};

mod common;

use moongraphql_builder::prelude::*;

// Bind Microsoft mimalloc memory allocator for concurrent Tokio thread acceleration
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// =========================================================================
// 🧱 TEST SELECTORS AND QUERY BUILDERS
// =========================================================================

// Nested poster field selector
define_selector! {
    /// Test selector for poster image assets.
    #[default_complexity = 5]
    #[default = "originalUrl mainUrl"]
    pub struct MockPosterSelector {
        /// Unique poster primary key.
        #[complexity = 1]
        id: id,
        /// Direct URL to original resolution asset.
        #[complexity = 1]
        original_url: originalUrl,
        /// Direct URL to standard display asset.
        #[complexity = 1]
        main_url: mainUrl,
    }
}

// Root entity field selector
define_selector! {
    /// Test selector for anime root entities.
    #[default_complexity = 1]
    #[default = "id name russian"]
    pub struct MockAnimeSelector {
        /// Unique numeric title identifier.
        #[complexity = 1]
        id: id,
        /// Original title in romaji or English.
        #[complexity = 1]
        name: name,
        /// Officially localized title name in Russian.
        #[complexity = 1]
        russian: russian,
        /// Average user rating score.
        #[complexity = 1]
        score: score,
        /// Nested poster image object.
        #[complexity = 5]
        poster(MockPosterSelector): poster,
    }
}

/// Test sort order enumeration supporting [`std::fmt::Display`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MockOrder {
    /// Sort by ranked score.
    Ranked,
    /// Sort by popularity count.
    Popularity,
}

impl std::fmt::Display for MockOrder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ranked => write!(f, "ranked"),
            Self::Popularity => write!(f, "popularity"),
        }
    }
}

/// Custom validation predicate verifying that a string is not blank using [`ArgMeta`].
///
/// # Arguments
///
/// * `val` ([`&String`]) — Candidate search string.
/// * `meta` ([`&ArgMeta`]) — Metadata descriptor carrying parameter and root query names.
///
/// # Returns
///
/// Returns `Ok(())` if non-blank.
///
/// # Errors
///
/// Emits an error string if `val` consists entirely of whitespace.
fn validate_not_blank(val: &String, meta: &ArgMeta) -> Result<(), String> {
    if val.trim().is_empty() {
        return Err(format!(
            "Argument '{}' in query '{}' cannot consist solely of whitespace",
            meta.arg_name, meta.root_query
        ));
    }
    Ok(())
}

// Declarative query builder generation with hybrid argument validation
define_query_builder! {
    /// Test query builder for 'animes' with declarative argument validation.
    #[root = animes]
    #[selector = MockAnimeSelector]
    pub struct MockAnimesQueryBuilder {
        /// Mandatory target page index (minimum 1).
        #[validate(required)]
        #[validate(min = 1)]
        page: u32 => page("PositiveInt", scalar),

        /// Target items limit (bounded to 1..=50).
        #[validate(range = 1..=50)]
        limit: u32 => limit("PositiveInt", scalar),

        /// Upper score bound (maximum 10).
        #[validate(max = 10)]
        score: u32 => score("Int", scalar),

        /// Sort order enumeration.
        order: MockOrder => order("OrderEnum", display),

        /// Textual search query: length check (2..=100) + custom non-blank predicate.
        #[validate(min_len = 2)]
        #[validate(max_len = 100)]
        #[validate(custom = validate_not_blank)]
        search: String => search("String", string),

        /// Censor flag for age-restricted material.
        is_censored: bool => censored("Boolean", scalar),

        /// Exact entity identifier filter (up to 5 elements).
        #[validate(max_len = 5)]
        ids: Vec<String> => ids("[ID!]", list),
    }
}

/// Deserializable mock payload model for JSON verification.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
struct MockAnimeData {
    /// Entity identifier.
    id: u32,
    /// Original title name.
    name: String,
    /// Localized Russian title name.
    russian: Option<String>,
}

// =========================================================================
// 🧩 SECTION 1: FIELD SELECTORS & TYPE-STATE DEDUPLICATION
// =========================================================================

/// Verifies deterministic [`std::collections::BTreeSet`] field sorting and duplicate elimination.
#[test]
fn it_test_selector_field_deduplication_and_sorting_ok() {
    common::setup_test_environment();

    let selector = MockAnimeSelector::new().score().name().id().russian();

    // Fields must be compiled in strict alphabetical order
    assert_eq!(selector.build(), "id name russian score");
}

/// Verifies default field set fallback when no selector methods were invoked.
#[test]
fn it_test_selector_default_fields_when_empty_ok() {
    common::setup_test_environment();

    let selector = MockAnimeSelector::new();
    assert_eq!(selector.build(), "id name russian");
    // 1 (root) + 3 (default fields: id, name, russian) = 4
    assert_eq!(selector.complexity(), 4);
    assert_eq!(selector.depth(), 1);
}

/// Verifies closure parameter variations: custom field picks vs default identity closure `|p| p`.
#[test]
fn it_test_selector_closure_variants_ok() {
    common::setup_test_environment();

    // 1. Via custom closure with inferred `|p|` type
    let custom_sel = MockAnimeSelector::new().poster(|p| p.id());
    assert_eq!(custom_sel.build(), "poster { id }");

    // 2. Via identity closure `|p| p` - automatically expands default poster fields
    let default_sel = MockAnimeSelector::new().poster(|p| p);
    assert_eq!(default_sel.build(), "poster { originalUrl mainUrl }");
}

// =========================================================================
// 📊 SECTION 2: COMPLEXITY & AST DEPTH METRICS
// =========================================================================

/// Verifies exact calculation of nested AST complexity and tree depth.
#[test]
fn it_test_nested_complexity_and_depth_calculation_ok() {
    common::setup_test_environment();

    let selector = MockAnimeSelector::new()
        .id()
        .name()
        .poster(|p| p.original_url().main_url());

    // Complexity calculation breakdown:
    // Root MockAnimeSelector base cost: 1
    // Field id: 1
    // Field name: 1
    // Field poster (#[complexity = 5]): 5
    // Nested poster fields: originalUrl (1) + mainUrl (1) = 2
    // Total: 1 + 1 + 1 + 5 + 2 = 10
    assert_eq!(selector.complexity(), 10);
    assert_eq!(selector.fields_complexity(), 9);
    assert_eq!(selector.depth(), 2);
}

// =========================================================================
// 🚀 SECTION 3: QUERY BUILDER GENERATION
// =========================================================================

/// Verifies inline query generation via [`define_query_builder!`].
#[test]
fn it_test_query_builder_inline_generation_ok() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .search("Overlord")
        .limit(10)
        .order(MockOrder::Ranked)
        .is_censored(true)
        .select(|f| f.id().name().score());

    let compiled = query._build_query();
    assert_eq!(
        compiled,
        "query { animes(page: 1, limit: 10, order: \"ranked\", search: \"Overlord\", censored: true) { id name score } }"
    );
    // Complexity: 1 (root animes) + 3 (fields) = 4
    assert_eq!(query.complexity(), 4);
    assert_eq!(query.depth(), 1);
}

/// Verifies automatic variable extraction and JSON variables generation.
#[test]
fn it_test_query_builder_variables_and_json_generation_ok() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .limit(5)
        .search("Bakemonogatari")
        .ids(["101", "102"])
        .select(|f| f.id().name().poster(|p| p.original_url()));

    let (query_body, decls, vars) = query._build_query_parts(true);
    let full_doc = format!("query ({}) {{ {} }}", decls.join(", "), query_body);

    // Verify GraphQL document envelope with variable declarations
    assert_eq!(
        full_doc,
        "query ($animes_page: PositiveInt, $animes_limit: PositiveInt, $animes_search: String, $animes_ids: [ID!]) { animes(page: $animes_page, limit: $animes_limit, search: $animes_search, ids: $animes_ids) { id name poster { originalUrl } } }"
    );

    // Verify JSON variable bindings
    assert_eq!(vars.get("animes_page").unwrap(), &serde_json::json!(1));
    assert_eq!(vars.get("animes_limit").unwrap(), &serde_json::json!(5));
    assert_eq!(
        vars.get("animes_search").unwrap(),
        &serde_json::json!("Bakemonogatari")
    );
    assert_eq!(
        vars.get("animes_ids").unwrap(),
        &serde_json::json!(["101", "102"])
    );
}

// =========================================================================
// 🛡️ SECTION 4: METRIC VALIDATOR
// =========================================================================

/// Verifies successful validation pass when all metrics are within bounds.
#[test]
fn it_test_metric_validator_within_limits_ok() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .select(|f| f.id().name());

    let validator = MetricValidator::new(190, 5);
    assert!(validator.validate(&query).is_ok());
}

/// Verifies error interception when query complexity exceeds threshold ([`MoongqlError::ComplexityExceeded`]).
#[test]
fn it_test_metric_validator_complexity_exceeded_err() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .select(|f| f.id().name().poster(|p| p.original_url().main_url()));

    // Set limit to 8 (calculated is 10)
    let validator = MetricValidator::new(8, 5);
    let result = validator.validate(&query);

    assert_eq!(
        result,
        Err(MoongqlError::ComplexityExceeded {
            calculated: 10,
            limit: 8,
        })
    );
}

/// Verifies error interception when nesting depth exceeds threshold ([`MoongqlError::DepthExceeded`]).
#[test]
fn it_test_metric_validator_depth_exceeded_err() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .select(|f| f.poster(|p| p.original_url()));

    // Set depth limit to 1 (calculated is 2)
    let validator = MetricValidator::new(190, 1);
    let result = validator.validate(&query);

    assert_eq!(
        result,
        Err(MoongqlError::DepthExceeded {
            calculated: 2,
            limit: 1,
        })
    );
}

/// Verifies local query-level overrides superseding global validator limits.
#[test]
fn it_test_query_override_limits_ok() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .with_max_complexity(500)
        .with_max_depth(10)
        .select(|f| f.id().name().poster(|p| p.original_url()));

    // Global limits are strict (5 points, depth 1), but local overrides (500, 10) must prevail
    let validator = MetricValidator::new(5, 1);
    assert!(validator.validate(&query).is_ok());
}

// =========================================================================
// 🌍 SECTION 5: INTERNATIONALIZATION (FLUENT I18N)
// =========================================================================

/// Verifies dual-locale error rendering in Russian and English.
#[test]
fn it_test_i18n_error_localization_ru_en_ok() {
    common::setup_test_environment();

    let err = MoongqlError::ComplexityExceeded {
        calculated: 250,
        limit: 190,
    };

    // 1. Verify Russian localization
    set_global_locale("ru");
    assert_eq!(
        err.to_string(),
        "❌ Превышен лимит сложности GraphQL-запроса: рассчитано 250, максимально допустимо 190"
    );

    // 2. Verify English localization
    set_global_locale("en");
    assert_eq!(
        err.to_string(),
        "❌ GraphQL query complexity limit exceeded: calculated 250, max allowed 190"
    );

    // Restore Russian default locale
    set_global_locale("ru");
}

/// Verifies dynamic runtime registration of external FTL bundles.
#[test]
fn it_test_i18n_register_external_resource_ok() {
    common::setup_test_environment();

    // Register custom external translation keys
    register_resource("ru", "custom-msg = 🚀 Привет из внешнего крейта: {$name}!");
    register_resource("en", "custom-msg = 🚀 Hello from external crate: {$name}!");

    set_global_locale("ru");
    let ru_msg = tr!("custom-msg", "name" => "Dina");
    assert_eq!(ru_msg, "🚀 Привет из внешнего крейта: Dina!");

    set_global_locale("en");
    let en_msg = tr!("custom-msg", "name" => "Dina");
    assert_eq!(en_msg, "🚀 Hello from external crate: Dina!");

    set_global_locale("ru");
}

// =========================================================================
// 📦 SECTION 6: PAYLOAD & SERVER RESPONSE HANDLING
// =========================================================================

/// Verifies [`GraphQLPayload`] serialization and [`GraphQLResponse`] deserialization.
#[test]
fn it_test_payload_and_response_serde_ok() {
    common::setup_test_environment();

    // 1. Verify JSON payload generation
    let payload = GraphQLPayload {
        operation_name: Some("GetAnimes".to_string()),
        query: "query GetAnimes { animes { id } }".to_string(),
        variables: Some(serde_json::json!({ "limit": 10 })),
    };
    let json_payload = serde_json::to_string(&payload).unwrap();
    assert!(json_payload.contains("\"operationName\":\"GetAnimes\""));

    // 2. Verify deserialization of successful response envelope
    let raw_success_json = r#"{
        "data": {
            "id": 1,
            "name": "Bakemonogatari",
            "russian": "Истории монстров"
        }
    }"#;
    let response: GraphQLResponse<MockAnimeData> = serde_json::from_str(raw_success_json).unwrap();
    assert!(response.errors.is_none());
    let data = response.data.unwrap();
    assert_eq!(data.name, "Bakemonogatari");
    assert_eq!(data.russian, Some("Истории монстров".to_string()));

    // 3. Verify deserialization of GraphQL error responses
    let raw_error_json = r#"{
        "errors": [
            {
                "message": "Query has complexity of 248, which exceeds max complexity of 190",
                "locations": [{"line": 1, "column": 7}]
            }
        ]
    }"#;
    let err_response: GraphQLResponse<MockAnimeData> =
        serde_json::from_str(raw_error_json).unwrap();
    assert!(err_response.data.is_none());
    let errors = err_response.errors.unwrap();
    assert_eq!(errors.len(), 1);
    assert_eq!(
        errors[0].message,
        "Query has complexity of 248, which exceeds max complexity of 190"
    );
    assert_eq!(errors[0].locations.as_ref().unwrap()[0].line, 1);
}

// =========================================================================
// 🎯 SECTION 7: HYBRID ARGUMENT VALIDATION (#[validate])
// =========================================================================

/// Verifies successful validation pass when all constraints are satisfied.
#[test]
fn it_test_argument_validation_success_ok() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .limit(20)
        .search("Bakemonogatari")
        .ids(["1", "2", "3"]);

    // Direct invocation of argument validation
    assert_eq!(query.validate_args(), Ok(()));

    // Integrated check via MetricValidator (arguments + AST metrics)
    let validator = MetricValidator::new(190, 5);
    assert!(validator.validate(&query).is_ok());
}

/// Verifies error interception for missing mandatory arguments (`#[validate(required)]`).
#[test]
fn it_test_argument_validation_required_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    // Instantiated without .page(...)
    let query = MockAnimesQueryBuilder::new().search("Bakemonogatari");

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Обязательный аргумент 'page' не указан в запросе 'animes'".to_string()
        ))
    );
}

/// Verifies error interception for lower numeric bounds (`#[validate(min = 1)]`).
#[test]
fn it_test_argument_validation_min_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    let query = MockAnimesQueryBuilder::new().page(0); // Violates min = 1

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Аргумент 'page' в запросе 'animes' должен быть не менее 1 (получено: 0)".to_string()
        ))
    );
}

/// Verifies error interception for range constraints (`#[validate(range = 1..=50)]`).
#[test]
fn it_test_argument_validation_range_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    let query = MockAnimesQueryBuilder::new().page(1).limit(100); // Violates range = 1..=50

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Аргумент 'limit' в запросе 'animes' должен быть в диапазоне 1..=50 (получено: 100)"
                .to_string()
        ))
    );
}

/// Verifies error interception for minimum string length (`#[validate(min_len = 2)]`).
#[test]
fn it_test_argument_validation_min_len_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    let query = MockAnimesQueryBuilder::new().page(1).search("A"); // Length 1, violates min_len = 2

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Длина аргумента 'search' в запросе 'animes' должна быть не менее 2 (получено: 1)"
                .to_string()
        ))
    );
}

/// Verifies error interception for maximum list length (`#[validate(max_len = 5)]`).
#[test]
fn it_test_argument_validation_list_max_len_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    let query = MockAnimesQueryBuilder::new()
        .page(1)
        .ids(["1", "2", "3", "4", "5", "6"]); // 6 elements, violates max_len = 5

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Длина аргумента 'ids' в запросе 'animes' должна быть не более 5 (получено: 6)"
                .to_string()
        ))
    );
}

/// Verifies custom validation predicate `#[validate(custom = validate_not_blank)]` with [`ArgMeta`].
#[test]
fn it_test_argument_validation_custom_function_error() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new().page(1).search("    "); // Length >= 2, but whitespace only

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Argument 'search' in query 'animes' cannot consist solely of whitespace".to_string()
        ))
    );
}

/// Verifies that [`MetricValidator::validate`] aborts early on argument errors before metric evaluation.
#[test]
fn it_test_metric_validator_catches_invalid_args() {
    common::setup_test_environment();

    let query = MockAnimesQueryBuilder::new().page(1).limit(999); // Invalid limit argument

    let validator = MetricValidator::new(190, 5);
    let result = validator.validate(&query);

    // Fails fast on argument boundaries before computing graph AST complexity
    assert!(matches!(result, Err(MoongqlError::ValidationError(_))));
}

/// Verifies error interception for upper numeric bounds (`#[validate(max = 10)]`).
#[test]
fn it_test_argument_validation_max_error() {
    common::setup_test_environment();
    set_global_locale("ru");

    let query = MockAnimesQueryBuilder::new().page(1).score(15); // Violates max = 10

    let result = query.validate_args();
    assert_eq!(
        result,
        Err(MoongqlError::ValidationError(
            "Аргумент 'score' в запросе 'animes' должен быть не более 10 (получено: 15)"
                .to_string()
        ))
    );
}
