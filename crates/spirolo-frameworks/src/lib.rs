//! Framework recognisers: pure functions from a parsed file to `TestCase` rows.
//!
//! Each recogniser claims one framework family and, given a [`FileContext`], returns the
//! cases it declares. The model crate below cannot name these (it is the leaf of the
//! dependency chain), so the binary asks [`recognisers`] for the default set and injects
//! it into [`spirolo_model::Model::build_with`].
//!
//! [`FileContext`]: spirolo_model::FileContext

mod analysis;
mod chain;
mod config;
mod context;
mod countable;
mod imported_calls;
mod kind_code;
mod msw;
mod playwright;
mod saga;
mod snapshot;
mod vitest;

pub use config::Configs;
pub use imported_calls::imported_calls;
pub use msw::MswNock;
pub use playwright::Playwright;
pub use saga::Saga;
pub use snapshot::{read_snapshot, SnapshotFacts};
pub use vitest::VitestJest;

use spirolo_model::Recogniser;

/// The Vitest/Jest recogniser as a value, so [`recognisers`] can hand out a reference to
/// it without allocating one per call.
static VITEST_JEST: VitestJest = VitestJest;

/// The Playwright recogniser as a value, alongside [`VITEST_JEST`].
static PLAYWRIGHT: Playwright = Playwright;

/// The MSW/nock handler recogniser as a value, alongside the test-case recognisers.
static MSW_NOCK: MswNock = MswNock;

/// The runner/mutation config recogniser as a value, alongside the rest.
static CONFIGS: Configs = Configs;

/// The saga step-walk recogniser as a value, alongside the rest.
static SAGA: Saga = Saga;

/// Every recogniser the binary runs, in the order their rows are merged (before the
/// model re-sorts, so the order is not observable in the output).
pub fn recognisers() -> Vec<&'static dyn Recogniser> {
    vec![&VITEST_JEST, &PLAYWRIGHT, &MSW_NOCK, &CONFIGS, &SAGA]
}
