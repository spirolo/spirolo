//! `spirolo-bench tally`: the `counts` a new corpus entry's `truth.json` records,
//! tallied by the same code the counts section compares them with. The document is a real
//! `spirolo list --format json` of the onboarding recipe's pnpm fixture
//! (`corpus/onboard/fixtures/pnpm-vitest`).

use serde_json::{json, Value};

fn fixture() -> Value {
    let text = include_str!("fixtures/onboard-pnpm-vitest-list.json");
    serde_json::from_str(text).unwrap()
}

#[test]
fn the_tally_is_the_truth_counts_shape_of_by_framework() {
    let doc = fixture();
    let tally = spirolo_bench::tally(&doc);
    let by_framework = spirolo_bench::by_framework(&doc, &[], &[]);
    assert!(!by_framework.is_empty());
    for (framework, count) in &by_framework {
        assert_eq!(
            tally[framework],
            json!({ "files": count.files, "tests": count.tests }),
            "{framework}"
        );
    }
    assert_eq!(tally.as_object().unwrap().len(), by_framework.len());
}
