//! Parsers for artifacts other tools write: the mutation report reader, the run report
//! readers (vitest JSON and `JUnit` XML) and the churn reader (`git log --numstat`).
//!
//! This crate reads bytes and returns rows; it never touches the filesystem, the network
//! or the clock. The binary reads the files its config names and hands the bytes here, so
//! a report the reader cannot use becomes a diagnostic rather than an aborted run.
//!
//! It sits above `spirolo-model` in the workspace chain
//! (`model ← frameworks ← inputs ← rules ← report ← spirolo`): it produces the model's
//! `Mutants`, `MutationTest`, `MutationReport`, `Run` and `Churn` rows, and the model carries them.

mod churn;
mod ledger;
mod mutation;
mod quarantine;
mod runs;

pub use crate::churn::{read_numstat, ChurnTable};
pub use crate::ledger::{
    content_run_id, read_run_file, read_sidecar, RunFile, RunFileTest, Sidecar, SidecarHome,
};
pub use crate::mutation::{read_mutation_report, MutationTables};
pub use crate::quarantine::{Added, QuarantineEntry, QuarantineFile, TreeEvidence};
pub use crate::runs::{read_junit_xml, read_vitest_json, RunContext, RunTables};
