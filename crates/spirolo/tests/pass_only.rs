//! The default view shows only rules shown by default (proven or advisory in the corpus
//! status, #518); `--all` shows every rule (#348). Driven through the real binary on the
//! captured email-kit engine, with the CLI's own table deciding which rows are hidden.

use std::path::PathBuf;

use assert_cmd::Command;
use serde_json::Value;

fn spirolo() -> Command {
    Command::cargo_bin("spirolo").unwrap()
}

fn engine() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/redundant-test-email-kit-engine")
}

/// `(rule, n, dropped_or_failed)` for each rule `check --all --format json` reports that the
/// CLI's own table hides by default, in registry order.
fn hidden_by_rule(all_json: &Value) -> Vec<(&'static str, usize, bool)> {
    let output = spirolo()
        .args(["rule-status", "--format", "json"])
        .output()
        .unwrap();
    let table = spirolo_rules::status::parse(std::str::from_utf8(&output.stdout).unwrap()).unwrap();
    spirolo_rules::registry()
        .iter()
        .map(|rule| rule.name())
        .filter_map(|name| {
            let tier = table.rules.get(name).map_or(
                spirolo_rules::status::Tier::Experimental,
                spirolo_rules::status::RuleStatus::tier,
            );
            let count = all_json["findings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|finding| finding["rule"] == name)
                .count();
            (!tier.shown_by_default() && count > 0).then_some((
                name,
                count,
                tier == spirolo_rules::status::Tier::Hidden,
            ))
        })
        .collect()
}

/// The ` hidden` line the rows should give: experimental rows, then DROPPED and FAIL rows,
/// each counted by rule, then the command that shows them.
fn hidden_line(hidden: &[(&str, usize, bool)], command: &str) -> String {
    let groups: Vec<String> = [
        (false, "experimental"),
        (true, "from DROPPED or FAIL rules"),
    ]
    .into_iter()
    .filter_map(|(dropped, what)| {
        let rows: Vec<_> = hidden.iter().filter(|row| row.2 == dropped).collect();
        let total: usize = rows.iter().map(|row| row.1).sum();
        let by_rule: Vec<_> = rows
            .iter()
            .map(|(name, count, _)| format!("{name} {count}"))
            .collect();
        (total > 0).then(|| format!("{total} {what} ({})", by_rule.join(", ")))
    })
    .collect();
    let total: usize = hidden.iter().map(|row| row.1).sum();
    format!(
        " hidden {total} findings: {}; {command} --all shows them\n",
        groups.join(", ")
    )
}

#[test]
fn the_default_view_hides_rules_below_the_floor_and_counts_what_all_would_show() {
    let all = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(engine())
        .output()
        .unwrap();
    let all: Value = serde_json::from_slice(&all.stdout).unwrap();
    let hidden = hidden_by_rule(&all);
    assert!(
        !hidden.is_empty(),
        "the engine has findings from rules hidden by default"
    );

    let output = spirolo().arg("check").arg(engine()).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8(output.stdout).unwrap();
    for (name, _, _) in &hidden {
        assert!(
            !stdout.contains(&format!("● {name} ")),
            "{name} shows:\n{stdout}"
        );
        assert!(
            !stdout.contains(&format!("spirolo explain {name}")),
            "{stdout}"
        );
    }
    assert!(
        stdout.contains(&hidden_line(&hidden, "spirolo check")),
        "{stdout}"
    );
}

#[test]
fn under_rule_the_hidden_line_names_the_rule_command_that_shows_it() {
    let all = spirolo()
        .args(["check", "--all", "--format", "json"])
        .arg(engine())
        .output()
        .unwrap();
    let all: Value = serde_json::from_slice(&all.stdout).unwrap();
    let hidden = hidden_by_rule(&all);
    let Some(&row) = hidden.first() else {
        panic!("the engine has findings from rules hidden by default")
    };
    let rule = row.0;

    let output = spirolo()
        .args(["check", "--rule", rule])
        .arg(engine())
        .output()
        .unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains(&hidden_line(
            &[row],
            &format!("spirolo check --rule {rule}")
        )),
        "{stdout}"
    );
}
