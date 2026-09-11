//! Test harness helper module for environment initialization, fixtures, and logging.

use std::sync::Once;

/// Static guard for one-time logger initialization across concurrent Cargo test worker threads.
static LOGGER_INIT: Once = Once::new();

/// Prepares the test environment prior to test suite execution.
///
/// # Implementation Details
///
/// 1. Reads environment variables from a `.env` file via [`dotenvy`](https://docs.rs/dotenvy) if present.
/// 2. Initializes standard test logging via [`env_logger`](https://docs.rs/env_logger) with capture enabled
///    and [`log::LevelFilter::Debug`] verbosity threshold.
/// 3. Emits a confirmation log message upon successful setup.
pub fn setup_test_environment() {
    LOGGER_INIT.call_once(|| {
        dotenvy::dotenv().ok();

        env_logger::builder()
            .is_test(true)
            .filter_level(log::LevelFilter::Debug)
            .try_init()
            .ok();

        log::info!("🧪 MoonGraphQL Builder test environment successfully initialized!");
    });
}
