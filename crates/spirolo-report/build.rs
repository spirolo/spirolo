//! Lists the rules with a page under `docs/rules/`, so `explain` links only pages that exist.

use std::{env, fs, path::Path};

fn main() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/rules");
    println!("cargo:rerun-if-changed={}", docs.display());
    let mut rules: Vec<String> = fs::read_dir(&docs)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter_map(|entry| {
                    let name = entry.file_name().into_string().ok()?;
                    let rule = name.strip_suffix(".md")?;
                    (!rule.starts_with('_')).then(|| rule.to_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    rules.sort();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("rule_docs.rs");
    fs::write(out, format!("&{rules:?}")).unwrap();
}
