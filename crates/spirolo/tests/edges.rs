//! The workspace's one-way dependency edges, checked against cargo's own view of it.
//!
//! `model ← frameworks ← inputs ← rules ← report ← spirolo` is a stack of layers, not a
//! chain of edges: a crate may be used by any layer above it and must never reach a
//! layer below. This test fails the moment someone adds, say, `spirolo-rules` to
//! `spirolo-model`. The other direction of the same mistake — `spirolo-report` inside
//! `spirolo-model` — cargo itself refuses as a cyclic package dependency, so a reverse
//! `use` cannot compile either way.
//!
//! Dev-dependencies count. A test in a lower crate that wanted to snapshot a higher
//! crate's output would be a legitimate cargo pattern and a scope leak here: the lower
//! crate would start caring about an output format. Take the assertion as the rule.

/// Position in the chain. Lower may be depended upon by higher, never the reverse.
fn rank(crate_name: &str) -> Option<usize> {
    [
        "spirolo-model",
        "spirolo-frameworks",
        "spirolo-inputs",
        "spirolo-rules",
        "spirolo-report",
        "spirolo",
        "spirolo-bench",
    ]
    .iter()
    .position(|known| *known == crate_name)
}

#[test]
fn every_spirolo_crate_depends_only_on_crates_earlier_in_the_chain() {
    let metadata = workspace_metadata();
    let packages = metadata["packages"].as_array().unwrap();
    let members: Vec<&str> = metadata["workspace_members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_str().unwrap())
        .collect();

    let mut checked = 0;
    for package in packages {
        let id = package["id"].as_str().unwrap();
        if !members.contains(&id) {
            continue;
        }
        let name = package["name"].as_str().unwrap();
        let own_rank = rank(name).unwrap_or_else(|| panic!("unknown workspace crate {name}"));
        checked += 1;

        for dependency in package["dependencies"].as_array().unwrap() {
            let dependency_name = dependency["name"].as_str().unwrap();
            let kind = dependency["kind"].as_str().unwrap_or("normal");
            let Some(dependency_rank) = rank(dependency_name) else {
                continue;
            };
            assert!(
                dependency_rank < own_rank,
                "{name} has a {kind} dependency on {dependency_name}, which is not a lower \
                 layer of model ← frameworks ← inputs ← rules ← report ← spirolo"
            );
        }
    }

    assert_eq!(checked, 7, "every workspace crate is covered by this test");
}

fn workspace_metadata() -> serde_json::Value {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Cargo.toml");

    let output = std::process::Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .expect("cargo metadata runs");
    assert!(output.status.success(), "cargo metadata failed");

    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}
