//! Human display regressions over the captured email-kit project, through a real check.

use std::{path::PathBuf, time::Duration};

#[path = "../../spirolo-rules/tests/support/status.rs"]
mod captured_status;

#[test]
fn machine_formats_differ_from_all_only_by_omitting_rows_hidden_by_default() {
    use clap::Parser as _;
    let root = project("frozen-formats", 12);
    // The captured table proves some rules and hides others; orphan-test's INSUFFICIENT is
    // turned advisory, so the default view keeps proven and advisory rows and drops the rest.
    let mut table = captured_status::table().clone();
    *table.rules.get_mut("orphan-test").unwrap() = advisory();
    spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
        let run = super::try_build_run(Some(&root), None, None, None, true, None).unwrap();
        let shown: Vec<_> = run
            .findings
            .iter()
            .filter(|finding| spirolo_rules::status::shown_by_default(finding.rule))
            .cloned()
            .collect();
        assert!(shown.iter().any(|finding| finding.rule == "orphan-test"));
        assert!(!shown.is_empty() && shown.len() < run.findings.len());
        let render = |format: &str, findings: &[spirolo_rules::Finding]| match format {
            "json" => spirolo_report::to_check_json(&run.model, findings),
            "sarif" => spirolo_report::to_sarif(findings, super::VERSION),
            "markdown" => spirolo_report::render_check_markdown(findings),
            "agent" => spirolo_report::render_check_agent(findings),
            _ => spirolo_report::render_github_annotations(findings),
        };
        for baselined in [false, true] {
            if baselined {
                // An empty baseline surfaces every row, so both runs see the same findings.
                std::fs::create_dir_all(root.join(".spirolo")).unwrap();
                std::fs::write(
                    root.join(".spirolo/baseline.json"),
                    spirolo_report::Baseline::default().to_json(),
                )
                .unwrap();
            }
            for format in ["json", "sarif", "markdown", "agent", "github-annotations"] {
                let args = [
                    "spirolo",
                    "check",
                    root.to_str().unwrap(),
                    "--format",
                    format,
                    "--propose",
                ];
                let checked = |args: Vec<&str>| {
                    let super::Command::Check(check) =
                        super::Cli::try_parse_from(args).unwrap().command
                    else {
                        unreachable!()
                    };
                    super::render_check(&check.common, None, true).unwrap().0
                };
                assert_eq!(
                    checked(args.to_vec()),
                    render(format, &shown),
                    "{format}, baselined={baselined}"
                );
                assert_eq!(
                    checked(args.into_iter().chain(["--all"]).collect()),
                    render(format, &run.findings),
                    "{format} --all, baselined={baselined}"
                );
            }
        }
    });
}

#[test]
fn with_one_rule_pass_the_default_view_shows_only_that_rules_findings() {
    use clap::Parser as _;
    // The editor oracle's captured table at 521ec2a, with every PASS but weak-assertion's
    // turned INSUFFICIENT.
    let mut table = spirolo_rules::status::parse(include_str!(
        "../../../corpus/lsp/fixtures/rule-status-521ec2a.json"
    ))
    .unwrap();
    for (rule, row) in &mut table.rules {
        if rule != "weak-assertion" && row.verdict == spirolo_rules::status::Verdict::Pass {
            row.verdict = spirolo_rules::status::Verdict::Insufficient;
        }
    }
    let root = project("one-pass", 1);
    spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
        let checked = |format: &str| {
            let args = [
                "spirolo",
                "check",
                root.to_str().unwrap(),
                "--format",
                format,
            ];
            let super::Command::Check(check) = super::Cli::try_parse_from(args).unwrap().command
            else {
                unreachable!()
            };
            super::render_check(&check.common, None, false).unwrap().0
        };
        let json: serde_json::Value = serde_json::from_str(&checked("json")).unwrap();
        let rules: std::collections::BTreeSet<_> = json["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|finding| finding["rule"].as_str().unwrap())
            .collect();
        assert_eq!(rules, ["weak-assertion"].into());
        let human = checked("human");
        let headings: Vec<_> = human
            .lines()
            .filter(|line| line.starts_with("● "))
            .collect();
        assert_eq!(headings.len(), 1, "{human}");
        assert!(headings[0].starts_with("● weak-assertion "), "{human}");
        assert!(human.contains(
            " hidden 5 findings: 4 experimental (orphan-test 4), 1 from DROPPED or FAIL rules (untested-hotspot 1); spirolo check --all shows them\n"
        ), "{human}");
    });
}

/// An advisory row: UNADJUDICATED, 16 of 20 decided findings true, exactly the 0.80 floor.
fn advisory() -> spirolo_rules::status::RuleStatus {
    spirolo_rules::status::RuleStatus {
        measured_on: Vec::new(),
        verdict: spirolo_rules::status::Verdict::Unadjudicated,
        carried: None,
        precision: Some(0.8),
        interval: None,
        truths: 16,
        falses: 4,
    }
}

#[test]
fn advisory_rows_show_at_info_and_the_rest_wait_for_all() {
    use clap::Parser as _;
    use spirolo_rules::status::{Tier, Verdict};
    // The editor oracle's captured table at 521ec2a: weak-assertion's PASS turned advisory,
    // untested-hotspot FAIL as captured, and orphan-test first as captured (INSUFFICIENT,
    // 2 decided: experimental), then DROPPED.
    let root = project("tiers", 1);
    for (orphan, tier) in [
        (None, Tier::Experimental),
        (Some(Verdict::Dropped), Tier::Hidden),
    ] {
        let mut table = spirolo_rules::status::parse(include_str!(
            "../../../corpus/lsp/fixtures/rule-status-521ec2a.json"
        ))
        .unwrap();
        *table.rules.get_mut("weak-assertion").unwrap() = advisory();
        if let Some(verdict) = orphan {
            table.rules.get_mut("orphan-test").unwrap().verdict = verdict;
        }
        spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
            assert_eq!(
                spirolo_rules::status::tier("weak-assertion"),
                Tier::Advisory
            );
            assert_eq!(spirolo_rules::status::tier("orphan-test"), tier);
            assert_eq!(
                spirolo_rules::status::tier("untested-hotspot"),
                Tier::Hidden
            );
            let checked = |extra: &[&str]| {
                let args = ["spirolo", "check", root.to_str().unwrap()];
                let super::Command::Check(check) =
                    super::Cli::try_parse_from(args.iter().chain(extra))
                        .unwrap()
                        .command
                else {
                    unreachable!()
                };
                super::render_check(&check.common, None, false).unwrap()
            };
            let rules = |json: &str| {
                let json: serde_json::Value = serde_json::from_str(json).unwrap();
                json["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|finding| {
                        let rule = finding["rule"].as_str().unwrap().to_owned();
                        (rule, finding["severity"].as_str().unwrap().to_owned())
                    })
                    .collect::<std::collections::BTreeSet<_>>()
            };
            let (json, code) = checked(&["--format", "json"]);
            assert_eq!(
                rules(&json),
                [("weak-assertion".into(), "info".into())].into(),
                "{tier:?}"
            );
            assert_eq!(code, 0, "{tier:?}");
            let (human, code) = checked(&[]);
            assert!(human.contains("● weak-assertion "), "{human}");
            // The hidden line counts experimental rows apart from DROPPED and FAIL rows.
            let hidden = match tier {
                Tier::Experimental => {
                    "4 experimental (orphan-test 4), 1 from DROPPED or FAIL rules (untested-hotspot 1)"
                }
                _ => "5 from DROPPED or FAIL rules (orphan-test 4, untested-hotspot 1)",
            };
            assert!(
                human.contains(&format!(
                    " hidden 5 findings: {hidden}; spirolo check --all shows them\n"
                )),
                "{human}"
            );
            assert_eq!(code, 0, "{human}");
            let (json, code) = checked(&["--all", "--format", "json"]);
            let shown: std::collections::BTreeSet<_> =
                rules(&json).into_iter().map(|(rule, _)| rule).collect();
            assert_eq!(
                shown,
                ["orphan-test", "untested-hotspot", "weak-assertion"]
                    .map(String::from)
                    .into(),
                "{tier:?}"
            );
            assert_eq!(code, 0, "{tier:?}");
        });
    }
}

#[test]
fn a_dropped_rule_shows_under_all_with_its_status_and_never_sets_the_exit_code() {
    use clap::Parser as _;
    // The editor oracle's captured table at 521ec2a, with weak-assertion's PASS turned DROPPED.
    let mut table = spirolo_rules::status::parse(include_str!(
        "../../../corpus/lsp/fixtures/rule-status-521ec2a.json"
    ))
    .unwrap();
    table.rules.get_mut("weak-assertion").unwrap().verdict =
        spirolo_rules::status::Verdict::Dropped;
    let root = project("dropped", 1);
    // Even a config that asks the rule to fail the run cannot make a dropped rule gate.
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        r#"{ "rules": { "weak-assertion": { "severity": "error" } } }"#,
    )
    .unwrap();
    spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
        let checked = |extra: &[&str]| {
            let args = ["spirolo", "check", root.to_str().unwrap()];
            let super::Command::Check(check) = super::Cli::try_parse_from(args.iter().chain(extra))
                .unwrap()
                .command
            else {
                unreachable!()
            };
            super::render_check(&check.common, None, false).unwrap()
        };
        let (human, code) = checked(&["--all"]);
        let heading = human
            .lines()
            .find(|line| line.starts_with("● weak-assertion "))
            .unwrap_or_else(|| panic!("{human}"));
        assert!(heading.ends_with(" · rule: DROPPED"), "{heading}");
        // No severity override can gate a dropped rule, so the view never offers one.
        assert!(
            !human.contains(r#""weak-assertion": { "severity""#),
            "{human}"
        );
        assert_eq!(code, 0, "{human}");
        let (human, code) = checked(&[]);
        assert!(!human.contains("● weak-assertion "), "{human}");
        assert_eq!(code, 0, "{human}");
    });
}

#[test]
fn a_gating_override_opts_an_advisory_or_experimental_rule_in_and_never_a_dropped_or_fail_one() {
    use clap::Parser as _;
    use spirolo_rules::status::{Tier, Verdict};
    // The editor oracle's captured table at 521ec2a: weak-assertion's PASS turned advisory,
    // orphan-test as captured (INSUFFICIENT, 2 decided: experimental) or turned DROPPED, and
    // untested-hotspot FAIL as captured.
    let root = project("overrides", 1);
    for (rule, orphan, tier, gates) in [
        ("weak-assertion", None, Tier::Advisory, true),
        ("orphan-test", None, Tier::Experimental, true),
        ("orphan-test", Some(Verdict::Dropped), Tier::Hidden, false),
        ("untested-hotspot", None, Tier::Hidden, false),
    ] {
        let mut table = spirolo_rules::status::parse(include_str!(
            "../../../corpus/lsp/fixtures/rule-status-521ec2a.json"
        ))
        .unwrap();
        *table.rules.get_mut("weak-assertion").unwrap() = advisory();
        if let Some(verdict) = orphan {
            table.rules.get_mut("orphan-test").unwrap().verdict = verdict;
        }
        std::fs::write(
            root.join(".spirolorc.jsonc"),
            format!(r#"{{ "rules": {{ "{rule}": {{ "severity": "warn" }} }} }}"#),
        )
        .unwrap();
        spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
            assert_eq!(spirolo_rules::status::tier(rule), tier);
            let checked = |extra: &[&str]| {
                let args = ["spirolo", "check", root.to_str().unwrap()];
                let super::Command::Check(check) =
                    super::Cli::try_parse_from(args.iter().chain(extra))
                        .unwrap()
                        .command
                else {
                    unreachable!()
                };
                super::render_check(&check.common, None, false).unwrap()
            };
            let severities = |json: &str| {
                let json: serde_json::Value = serde_json::from_str(json).unwrap();
                json["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|finding| finding["rule"] == rule)
                    .map(|finding| finding["severity"].as_str().unwrap().to_owned())
                    .collect::<std::collections::BTreeSet<_>>()
            };
            let note = format!(" note   config severity for {rule} not applied: rule is ");
            let hint = format!(r#"To gate it: .spirolorc.jsonc {{ "rules": {{ "{rule}""#);
            let heading = format!("● {rule} ");
            // The default view: an override that gates shows the rule's rows and sets the exit
            // code; one on a DROPPED or FAIL rule leaves them hidden at info and says so.
            let (json, code) = checked(&["--format", "json"]);
            let expected = if gates { "warn" } else { "info" };
            if gates {
                assert_eq!(severities(&json), [expected.to_owned()].into(), "{rule}");
            } else {
                assert!(severities(&json).is_empty(), "{rule}: {json}");
            }
            assert!(!json.contains("not applied"), "{json}");
            assert_eq!(code, u8::from(gates), "{rule}");
            let (human, code) = checked(&[]);
            assert_eq!(human.contains(&heading), gates, "{human}");
            if gates {
                assert!(!human.contains(&note), "{human}");
            } else {
                let verdict = spirolo_rules::status::status(rule).verdict.name();
                assert_eq!(
                    human.matches(&format!("{note}{verdict}\n")).count(),
                    1,
                    "{human}"
                );
            }
            assert_eq!(code, u8::from(gates), "{human}");
            // The editor's check takes the same view.
            let lsp = super::checked(&root, false).unwrap();
            assert_eq!(lsp.findings.iter().any(|f| f.rule == rule), gates, "{rule}");
            // `--all` shows every row; a DROPPED or FAIL rule's still report at info, with no
            // offer of an override that could not gate it (#506), and the notice still shows.
            let (json, code) = checked(&["--all", "--format", "json"]);
            assert_eq!(severities(&json), [expected.to_owned()].into(), "{rule}");
            assert_eq!(code, u8::from(gates), "{rule}");
            let (human, code) = checked(&["--all"]);
            assert!(human.contains(&heading), "{human}");
            assert!(!human.contains(&hint), "{human}");
            assert_eq!(human.contains(&note), !gates, "{human}");
            assert_eq!(code, u8::from(gates), "{human}");
        });
    }
}

#[test]
fn closing_line_leads_with_decided_findings_and_silent_rules_collapse_in_registry_order() {
    let root = project("tally", 1);
    let run = super::try_build_run(Some(&root), None, None, None, false, None).unwrap();
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    let tally = spirolo_report::tally(&run.ran, &run.findings);
    assert!(report.contains(&format!(
        "{} findings in {} of {} rules · 0 warn · 0 error",
        tally.info,
        tally.rules - tally.ok - tally.undecided,
        tally.rules
    )));
    let silent: Vec<_> = run
        .ran
        .iter()
        .filter(|r| !run.findings.iter().any(|f| f.rule == r.name()))
        .map(|r| r.name())
        .collect();
    assert!(report.contains(&format!(
        "✓ {} rules found nothing: {}",
        silent.len(),
        silent.join(", ")
    )));
    let run =
        super::try_build_run(Some(&root), None, Some("dead-skip"), None, false, None).unwrap();
    assert!(run.findings.is_empty());
    let clean =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    assert!(clean.contains("✓ 1 rules found nothing: dead-skip"));
    assert!(clean.contains("0 findings in 0 of 1 rules · 0 warn · 0 error · exit 0"));
    assert!(!clean.contains("── "));
}

#[test]
fn numeric_scores_rank_files_and_human_rows_drop_only_the_leading_path() {
    let root = project("scores", 2);
    std::fs::create_dir_all(root.join("zz")).unwrap();
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/m6-subject/src/utils/payment-analytics.ts"),
        root.join("zz/payment-analytics.ts"),
    )
    .unwrap();
    let run = super::try_build_run(
        Some(&root),
        None,
        Some("untested-hotspot"),
        None,
        false,
        None,
    )
    .unwrap();
    let mut ranked: Vec<_> = run.findings.iter().collect();
    let score = |f: &spirolo_rules::Finding| {
        f.evidence
            .iter()
            .find(|e| e.key == "score")
            .unwrap()
            .value
            .parse::<f64>()
            .unwrap()
    };
    ranked.sort_by(|a, b| {
        score(b)
            .total_cmp(&score(a))
            .then_with(|| a.location.file.cmp(&b.location.file))
    });
    assert!(ranked.len() > 1 && score(ranked[0]) > score(ranked[ranked.len() - 1]));
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    let rows: Vec<_> = report
        .lines()
        .filter(|line| line.starts_with("  ") && line.contains(" scores "))
        .collect();
    let expected: Vec<_> = ranked
        .iter()
        .map(|f| {
            format!(
                "  {}:{} {}",
                f.location.file,
                f.location.line,
                f.message
                    .strip_prefix(&format!("{} ", f.location.file))
                    .unwrap()
            )
        })
        .collect();
    assert_eq!(rows, expected);
    assert!(!report.contains("rows above are in file order"));
}

#[test]
fn all_flag_alone_shows_every_rule_under_todays_caps() {
    use clap::Parser as _;
    let root = project("all-capped", 12);
    for command in ["check", "audit", "hygiene", "gate"] {
        let cli = super::Cli::try_parse_from(["spirolo", command, root.to_str().unwrap(), "--all"])
            .unwrap();
        let (args, family) = match cli.command {
            super::Command::Check(args) => (args.common, None),
            super::Command::Audit(args) => (args, Some(spirolo_rules::Family::Audit)),
            super::Command::Hygiene(args) => (args.common, Some(spirolo_rules::Family::Hygiene)),
            super::Command::Gate(args) => (args, Some(spirolo_rules::Family::Gate)),
            _ => unreachable!(),
        };
        let (report, _) = super::render_check(&args, family, false).unwrap();
        // Today's view: every rule's rows, capped, without a hidden line.
        let run = super::try_build_run(Some(&root), None, None, family, false, None).unwrap();
        let today = spirolo_report::render_check_human_with_options(
            &run.model,
            Duration::ZERO,
            &run.ran,
            &run.findings,
            spirolo_report::HumanOptions {
                family,
                ..Default::default()
            },
        );
        let body = |view: &str| view.lines().skip(1).collect::<Vec<_>>().join("\n");
        assert_eq!(body(&report), body(&today), "{command} --all");
    }
}

#[test]
fn all_flag_with_a_rule_lifts_both_caps_and_baseline_uses_the_same_writer() {
    use clap::Parser as _;
    let root = project("all", 12);
    for (command, rule) in [
        ("check", "weak-assertion"),
        ("audit", "weak-assertion"),
        ("hygiene", "orphan-test"),
        ("gate", "gate-truth"),
    ] {
        let cli = super::Cli::try_parse_from([
            "spirolo",
            command,
            root.to_str().unwrap(),
            "--rule",
            rule,
            "--all",
        ])
        .unwrap();
        let (args, family) = match cli.command {
            super::Command::Check(args) => (args.common, None),
            super::Command::Audit(args) => (args, Some(spirolo_rules::Family::Audit)),
            super::Command::Hygiene(args) => (args.common, Some(spirolo_rules::Family::Hygiene)),
            super::Command::Gate(args) => (args, Some(spirolo_rules::Family::Gate)),
            _ => unreachable!(),
        };
        let (report, _) = super::render_check(&args, family, false).unwrap();
        assert!(
            !report.contains("… and"),
            "{command} --rule {rule} --all must lift both caps"
        );
        if command == "check" {
            let run =
                super::try_build_run(Some(&root), None, Some(rule), None, false, None).unwrap();
            assert_eq!(
                report
                    .lines()
                    .filter(|line| line.starts_with("    :")
                        || (line.starts_with("  ") && line.contains("/engine.ts:1"))
                        || line.starts_with("  project "))
                    .count(),
                run.findings.len()
            );
            std::fs::create_dir_all(root.join(".spirolo")).unwrap();
            std::fs::write(
                root.join(".spirolo/baseline.json"),
                spirolo_report::Baseline::default().to_json(),
            )
            .unwrap();
            let (baselined, _) = super::render_check(&args, family, false).unwrap();
            assert_eq!(
                baselined
                    .lines()
                    .filter(|line| line.starts_with("● ") || line.starts_with("    :"))
                    .collect::<Vec<_>>(),
                report
                    .lines()
                    .filter(|line| line.starts_with("● ") || line.starts_with("    :"))
                    .collect::<Vec<_>>()
            );
            std::fs::remove_file(root.join(".spirolo/baseline.json")).unwrap();
        }
    }
}

#[test]
fn ten_files_are_shown_and_project_rows_count_as_one_entry() {
    let root = project("file-cap", 12);
    let run =
        super::try_build_run(Some(&root), None, Some("weak-assertion"), None, false, None).unwrap();
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    assert!(report.contains("  project "));
    assert!(report.contains("  08/engine.test.ts (2)"));
    assert!(!report.contains("  09/engine.test.ts (2)"));
    assert!(report.contains("  … and 3 more files (spirolo check --rule weak-assertion --all)"));
    assert_eq!(
        report
            .lines()
            .filter(|line| line.starts_with("    :"))
            .count(),
        18
    );
}

#[test]
fn multiple_rows_in_one_file_are_capped_with_exact_overflow_count() {
    let root = clone_project("caps", 12);
    let run =
        super::try_build_run(Some(&root), None, Some("duplicate-test"), None, false, None).unwrap();
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    let mut files = std::collections::BTreeMap::<_, Vec<_>>::new();
    for finding in &run.findings {
        files
            .entry(&finding.location.file)
            .or_default()
            .push(finding);
    }
    assert_eq!(
        files.len(),
        1,
        "duplicate groups are anchored at their first member"
    );
    assert!(
        files.values().any(|rows| rows.len() > 5),
        "the real check must exceed the row cap"
    );
    for (file, rows) in &files {
        assert!(report.contains(&format!("  {file} ({})\n", rows.len())));
        if rows.len() > 5 {
            assert!(report.contains(&format!("    … and {} more\n", rows.len() - 5)));
        }
    }
    assert_eq!(
        report
            .lines()
            .filter(|line| line.starts_with("    :"))
            .count(),
        5
    );
}

/// The table `spirolo rule-status --format json` printed at 939f0af.
fn measured_939f0af() -> spirolo_rules::status::Table {
    spirolo_rules::status::parse(include_str!(
        "../../spirolo-rules/tests/fixtures/rule-status-939f0af.json"
    ))
    .unwrap()
}

#[test]
fn section_names_its_files_rows_claim_and_existing_actions() {
    // A section for an unproven rule that has not been dropped: weak-assertion's row as
    // measured at 939f0af with its verdict held at UNADJUDICATED, so neither the shipped table
    // nor its visit's verdict can move this heading (#501). A dropped rule's section is
    // `a_dropped_rule_shows_under_all_…`'s case.
    let mut table = measured_939f0af();
    table.rules.get_mut("weak-assertion").unwrap().verdict =
        spirolo_rules::status::Verdict::Unadjudicated;
    spirolo_rules::status::with_table(Box::leak(Box::new(table)), section_of_an_unproven_rule);
}

fn section_of_an_unproven_rule() {
    let root = project("section", 1);
    let run =
        super::try_build_run(Some(&root), None, Some("weak-assertion"), None, false, None).unwrap();
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    let files = run
        .findings
        .iter()
        .map(|f| &f.location.file)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    assert!(run.findings.iter().any(|f| f.location.file.is_empty()));
    assert_ne!(files, run.findings.len());
    let header = report
        .lines()
        .find(|line| line.starts_with("● weak-assertion"))
        .unwrap();
    // The heading says how far to trust the rule: 12 of 34 decided rows true, below the floor.
    assert_eq!(
        header,
        format!(
            "● weak-assertion ({files} files, {} findings) · experimental: 12/34 (UNADJUDICATED)",
            run.findings.len()
        )
    );
    assert!(report.contains(&format!(
        "  {} · spirolo explain weak-assertion",
        run.ran[0].summary()
    )));
    assert!(report.contains("To gate it: .spirolorc.jsonc { \"rules\": { \"weak-assertion\": { \"severity\": \"warn\" } } } · to accept today's rows: spirolo ratchet"));
}

#[test]
fn each_tier_labels_the_heading_with_how_far_to_trust_the_rule() {
    use spirolo_rules::status::Verdict;
    // weak-assertion's real rows, under rows from captured tables: the captured PASS (proven),
    // fail-closed-setup's at 939f0af (19/0, advisory), its own at 939f0af held UNADJUDICATED
    // (12/34, experimental), unused-mock's at 939f0af (nothing decided), and its own at
    // 939f0af as DROPPED and as FAIL.
    let measured = measured_939f0af();
    let held = |verdict| {
        let mut row = measured.rules["weak-assertion"].clone();
        row.verdict = verdict;
        row
    };
    let root = project("labels", 1);
    for (row, label) in [
        (captured_status::table().rules["weak-assertion"].clone(), ""),
        (
            measured.rules["fail-closed-setup"].clone(),
            " · advisory: 19/19 (UNADJUDICATED)",
        ),
        (
            held(Verdict::Unadjudicated),
            " · experimental: 12/34 (UNADJUDICATED)",
        ),
        (
            measured.rules["unused-mock"].clone(),
            " · experimental: not yet measured",
        ),
        (held(Verdict::Dropped), " · rule: DROPPED"),
        (held(Verdict::Fail), " · rule: FAIL"),
    ] {
        let mut table = measured_939f0af();
        table.rules.insert("weak-assertion".into(), row);
        spirolo_rules::status::with_table(Box::leak(Box::new(table)), || {
            let run =
                super::try_build_run(Some(&root), None, Some("weak-assertion"), None, false, None)
                    .unwrap();
            let heading = |report: &str| {
                report
                    .lines()
                    .find(|line| line.starts_with("● weak-assertion"))
                    .unwrap()
                    .to_owned()
            };
            let plain = spirolo_report::render_check_human(
                &run.model,
                Duration::ZERO,
                &run.ran,
                &run.findings,
            );
            assert_eq!(
                heading(&plain),
                format!("● weak-assertion (2 files, 3 findings){label}")
            );
            // The baselined view labels the same heading.
            let diff = spirolo_report::Baseline::default().diff(&run.findings);
            let baselined = spirolo_report::render_check_human_baselined(
                &run.model,
                Duration::ZERO,
                &run.ran,
                &diff,
                false,
            );
            assert_eq!(heading(&baselined), heading(&plain));
        });
    }
}

fn project(name: &str, copies: usize) -> PathBuf {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("display-305-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/redundant-test-email-kit-engine");
    for index in 0..copies {
        let dir = root.join(format!("{index:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        for file in ["engine.ts", "engine.test.ts"] {
            std::fs::copy(fixture.join(file), dir.join(file)).unwrap();
        }
    }
    unreached_module(&root);
    root
}

/// Copy email-kit's real `client.ts`, which no test reaches, into `root`: an
/// `untested-hotspot` row now that the engine's tests, which reference every value export,
/// keep `engine.ts` off the ranking (#456).
fn unreached_module(root: &std::path::Path) {
    std::fs::copy(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/redundant-test-email-kit/client.ts"),
        root.join("client.ts"),
    )
    .unwrap();
}

/// [`project`] plus copies of a test whose bodies read only imports of one shared module,
/// `clones/payment-analytics.ts`: the copies are the duplicates the real check groups, all
/// anchored in `clones/00/`. The engine copies are not duplicates of each other, since each
/// reads its own file-level helpers and runs under its own hooks.
fn clone_project(name: &str, copies: usize) -> PathBuf {
    let root = project(name, copies);
    let fixture =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-subject/src/utils");
    let clones = root.join("clones");
    std::fs::create_dir_all(&clones).unwrap();
    std::fs::copy(
        fixture.join("payment-analytics.ts"),
        clones.join("payment-analytics.ts"),
    )
    .unwrap();
    for index in 0..copies {
        let dir = clones.join(format!("{index:02}"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::copy(
            fixture.join("__tests__/payment-analytics.test.ts"),
            dir.join("payment-analytics.test.ts"),
        )
        .unwrap();
    }
    root
}

#[test]
fn sections_open_one_banner_per_nonempty_family_and_sort_rules_by_tier_inside_it() {
    let root = project("families", 2);
    let overrides: serde_json::Map<String, serde_json::Value> = spirolo_rules::registry()
        .iter()
        .map(|rule| (rule.name().into(), rule.default_severity().name().into()))
        .collect();
    std::fs::write(
        root.join(".spirolorc.jsonc"),
        serde_json::json!({"rules": overrides}).to_string(),
    )
    .unwrap();
    captured_status::with(|| {
        let run = super::try_build_run(Some(&root), None, None, None, false, None).unwrap();
        let report =
            spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
        let headings: Vec<_> = report
            .lines()
            .filter_map(|line| {
                line.strip_prefix("● ")
                    .map(|line| line.split_once(" (").unwrap().0)
            })
            .collect();
        let rule = |name: &str| run.ran.iter().find(|r| r.name() == name).unwrap();
        let tier = |name: &str| spirolo_rules::status::tier(name) as u8;
        let families: std::collections::BTreeSet<_> =
            headings.iter().map(|name| rule(name).family()).collect();
        assert!(
            families.len() >= 3,
            "the real check must span three families"
        );
        let key = |name: &str| {
            (
                rule(name).family(),
                tier(name),
                run.findings
                    .iter()
                    .filter(|f| f.rule == name)
                    .map(|f| f.severity)
                    .min()
                    .unwrap(),
                run.ran.iter().position(|r| r.name() == name).unwrap(),
            )
        };
        assert!(headings.windows(2).all(|pair| key(pair[0]) < key(pair[1])));
        for family in [
            spirolo_rules::Family::Audit,
            spirolo_rules::Family::Gate,
            spirolo_rules::Family::Hygiene,
            spirolo_rules::Family::Flake,
            spirolo_rules::Family::Impact,
        ] {
            let banner = format!("── {} ({}) ──", family.title(), family.name());
            assert_eq!(
                report
                    .lines()
                    .filter(|line| line.starts_with(&banner))
                    .count(),
                usize::from(families.contains(&family)),
                "{banner}"
            );
            if let Some(first) = headings.iter().find(|name| rule(name).family() == family) {
                assert!(
                    report.find(&banner).unwrap() < report.find(&format!("● {first} (")).unwrap()
                );
            }
        }
        assert!(!report.contains("── Advisory") && !report.contains("── Unproven"));
    });
}

#[test]
fn interleaved_real_findings_have_one_section_per_rule() {
    let root = clone_project("interleaved", 2);
    let run = super::try_build_run(Some(&root), None, None, None, false, None).unwrap();
    let mut locations: Vec<_> = run.findings.iter().collect();
    locations.sort_by_key(|f| (&f.location.file, f.location.line));
    assert!(
        locations
            .windows(3)
            .any(|rows| { rows[0].rule == rows[2].rule && rows[0].rule != rows[1].rule }),
        "the real check must exercise interleaved rules"
    );
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    for rule in &run.ran {
        if run.findings.iter().any(|f| f.rule == rule.name()) {
            assert_eq!(
                report
                    .lines()
                    .filter(|line| { line.starts_with(&format!("● {} (", rule.name())) })
                    .count(),
                1,
                "{} must have exactly one section",
                rule.name()
            );
        }
    }
}

/// Copy a whole committed fixture project into its own target directory, so a real check
/// runs on it without writing into the tree or colliding with another test's copy.
fn fixture_project(name: &str, fixture: &str) -> PathBuf {
    fn copy(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../target")
        .join(format!("display-312-{name}"));
    let _ = std::fs::remove_dir_all(&root);
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(fixture),
        &root,
    );
    root
}

#[test]
fn check_prints_the_health_line_under_the_header_without_a_proof_label() {
    let root = fixture_project("score-line", "redundant-test-email-kit-engine");
    captured_status::with(|| {
        let run = super::try_build_run(Some(&root), None, None, None, false, None).unwrap();
        let report =
            spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
        let health = spirolo_report::score::health(
            &run.findings,
            &run.model,
            spirolo_rules::status::table(),
        );
        assert!(health.provisional);
        assert_eq!(
            report.lines().nth(1).unwrap(),
            format!("Health {} ({})", health.grade, health.score.round())
        );
    });
}

#[test]
fn a_baselined_check_scores_only_the_findings_it_surfaces() {
    let root = fixture_project("score-baselined", "redundant-test-email-kit-engine");
    captured_status::with(|| {
        let run = super::try_build_run(Some(&root), None, None, None, false, None).unwrap();
        let diff = spirolo_report::Baseline::from_findings(&run.findings).diff(&run.findings);
        assert!(diff.surfaced.len() < run.findings.len());
        let report = spirolo_report::render_check_human_baselined(
            &run.model,
            Duration::ZERO,
            &run.ran,
            &diff,
            false,
        );
        let health = spirolo_report::score::health(
            &diff.surfaced,
            &run.model,
            spirolo_rules::status::table(),
        );
        assert_eq!(
            report.lines().nth(1).unwrap(),
            format!("Health {} ({})", health.grade, health.score.round())
        );
    });
}

#[test]
fn a_family_subcommand_prints_only_its_own_part_of_the_score() {
    let root = fixture_project("score-part", "redundant-test-email-kit-engine");
    captured_status::with(|| {
        let family = spirolo_rules::Family::Audit;
        let run = super::try_build_run(Some(&root), None, None, Some(family), false, None).unwrap();
        let report = spirolo_report::render_check_human_with_options(
            &run.model,
            Duration::ZERO,
            &run.ran,
            &run.findings,
            spirolo_report::HumanOptions {
                family: Some(family),
                ..Default::default()
            },
        );
        let health = spirolo_report::score::health(
            &run.findings,
            &run.model,
            spirolo_rules::status::table(),
        );
        let part = health.parts.iter().find(|p| p.family == "audit").unwrap();
        assert!(part.penalty > 0.0);
        assert_eq!(
            report.lines().nth(1).unwrap(),
            format!("Weak tests −{:.1} / {}", part.penalty, part.cap)
        );
        assert!(!report.contains("Health "));
    });
}

#[test]
fn the_more_files_hint_names_the_family_subcommand_that_ran() {
    let root = project("hint-hygiene", 12);
    let family = spirolo_rules::Family::Hygiene;
    let run = super::try_build_run(Some(&root), None, None, Some(family), false, None).unwrap();
    let report = spirolo_report::render_check_human_with_options(
        &run.model,
        Duration::ZERO,
        &run.ran,
        &run.findings,
        spirolo_report::HumanOptions {
            family: Some(family),
            ..Default::default()
        },
    );
    assert!(report.contains("  … and 2 more files (spirolo hygiene --rule orphan-test --all)"));
    assert!(!report.contains("(spirolo check --rule"));
}

#[test]
fn a_rule_with_only_undecided_rows_has_a_heading_and_counts_under_could_not_decide() {
    let root = fixture_project("undecided-only", "redundant-test-email-kit");
    let run = super::try_build_run(
        Some(&root),
        None,
        None,
        Some(spirolo_rules::Family::Hygiene),
        false,
        None,
    )
    .unwrap();
    let unused: Vec<_> = run
        .findings
        .iter()
        .filter(|f| f.rule == "unused-mock")
        .collect();
    assert!(
        !unused.is_empty()
            && unused
                .iter()
                .all(|f| f.outcome == spirolo_rules::Outcome::CouldNotDecide),
        "the real check must leave unused-mock undecided only"
    );
    let tally = spirolo_report::tally(&run.ran, &run.findings);
    assert_eq!((tally.ok, tally.undecided), (3, 1));
    let report =
        spirolo_report::render_check_human(&run.model, Duration::ZERO, &run.ran, &run.findings);
    assert!(report.contains("\n● unused-mock (1 file, 2 findings) · 2 could not decide"));
    assert!(
        report.contains("✓ 3 rules found nothing: duplicate-test, redundant-test, stale-snapshot")
    );
    assert!(report.contains(
        "3 findings in 1 of 5 rules · 1 rule could not decide · 0 warn · 0 error · 3 info · 2 could not decide · exit 0"
    ));
}

/// The human view a real run of `family` (`None` for `check`) prints over `root`, rendered
/// against the captured status table so a moved measurement does not move the snapshot.
fn human_view(root: &std::path::Path, family: Option<spirolo_rules::Family>) -> String {
    captured_status::with(|| {
        let run = super::try_build_run(Some(root), None, None, family, false, None).unwrap();
        spirolo_report::render_check_human_with_options(
            &run.model,
            Duration::ZERO,
            &run.ran,
            &run.findings,
            spirolo_report::HumanOptions {
                family,
                ..Default::default()
            },
        )
    })
}

#[test]
fn check_view_with_findings_in_three_families_is_stable() {
    let root = fixture_project("snap-check", "redundant-test-email-kit-engine");
    unreached_module(&root);
    insta::assert_snapshot!("human_check_three_families", human_view(&root, None));
}

#[test]
fn each_family_subcommand_view_is_stable() {
    use spirolo_rules::Family;
    for (family, fixture) in [
        (Family::Audit, "redundant-test-email-kit-engine"),
        (Family::Gate, "stale-snapshot-vitest-3.2.7"),
        (Family::Hygiene, "stale-snapshot-vitest-3.2.7"),
        (Family::Flake, "impact-runs-vitest-2.1.9"),
    ] {
        let root = fixture_project(&format!("snap-{}", family.name()), fixture);
        insta::assert_snapshot!(
            format!("human_{}", family.name()),
            human_view(&root, Some(family))
        );
    }
}

#[test]
fn the_hint_under_a_family_subcommand_is_stable() {
    let root = clone_project("snap-hint", 12);
    insta::assert_snapshot!(
        "human_hygiene_hint",
        human_view(&root, Some(spirolo_rules::Family::Hygiene))
    );
}

#[test]
fn an_undecided_only_rule_view_is_stable() {
    let root = fixture_project("snap-undecided", "redundant-test-email-kit");
    insta::assert_snapshot!(
        "human_hygiene_undecided_only",
        human_view(&root, Some(spirolo_rules::Family::Hygiene))
    );
}
