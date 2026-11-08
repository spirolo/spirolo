//! Flake extraction on source harvested from real projects.

use std::path::Path;

use spirolo_model::{BuildOptions, Enclosing, FlakeKind, FlakeNames, Model};

#[test]
fn a_real_while_true_poll_can_compute_its_deadline_comparison_inside_the_body() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-polls");
    let model = Model::build(&root, &BuildOptions::default());
    let clocks: Vec<_> = model
        .flake_sites
        .iter()
        .filter(|site| site.file == "retry-after.ts" && site.kind == FlakeKind::WallClock)
        .map(|site| site.span.line)
        .collect();
    assert!(
        clocks.is_empty(),
        "unexpected deadline clock sites: {clocks:?}"
    );
}

#[test]
fn a_real_do_while_poll_spaces_state_reads_and_uses_the_clock_only_as_a_deadline() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-polls");
    let model = Model::build(&root, &BuildOptions::default());
    let sites: Vec<_> = model
        .flake_sites
        .iter()
        .filter(|site| {
            site.file == "do-poll.ts"
                && matches!(site.kind, FlakeKind::WallClock | FlakeKind::FixedTimeout)
        })
        .map(|site| (site.kind, site.span.line))
        .collect();
    assert_eq!(sites, []);
}

#[test]
fn a_real_while_true_poll_uses_its_body_deadline_only_to_cap_state_reads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-polls");
    let model = Model::build(&root, &BuildOptions::default());
    let clocks: Vec<_> = model
        .flake_sites
        .iter()
        .filter(|site| site.file == "while-poll.ts" && site.kind == FlakeKind::WallClock)
        .map(|site| site.span.line)
        .collect();
    assert!(clocks.is_empty(), "unexpected clock sites: {clocks:?}");
}

#[test]
fn a_while_true_poll_can_break_on_state_instead_of_returning_it() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("state-break-while");
    std::fs::create_dir_all(&root).unwrap();
    // Both successful exits end the wait; keep the real poll's deadline unchanged.
    let source = include_str!("fixtures/m6-polls/while-poll.ts").replace(
        "if (connection) return connection;",
        "if (connection) break;",
    );
    std::fs::write(root.join("wait.ts"), source).unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    assert!(model
        .flake_sites
        .iter()
        .all(|site| site.kind != FlakeKind::WallClock));
}

#[test]
fn a_loop_with_only_a_timer_exit_still_depends_on_the_clock() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("timer-only-while");
    std::fs::create_dir_all(&root).unwrap();
    let source = include_str!("fixtures/m6-polls/while-poll.ts")
        .replace("    if (connection) return connection;", "");
    std::fs::write(root.join("wait.ts"), source).unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    assert!(
        model
            .flake_sites
            .iter()
            .any(|site| site.kind == FlakeKind::WallClock),
        "removing the state exit leaves a timer-dependent loop"
    );
}

#[test]
fn a_timer_only_do_while_with_a_body_clock_alias_still_records_its_wait_and_clocks() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("timer-only-do-while-alias");
    std::fs::create_dir_all(&root).unwrap();
    // Remove only the successful state exit from the real poll, then express its
    // deadline through an assigned local. The original literal sleep remains.
    let source = include_str!("fixtures/m6-polls/do-poll.ts")
        .replace("  let last: T;", "  let last: T;\n  let elapsed = 0;")
        .replace(
            "    if (predicate(last)) {\n      return last;\n    }",
            "    elapsed = Date.now() - deadline;",
        )
        .replace("Date.now() < deadline", "elapsed < 0");
    std::fs::write(root.join("wait.ts"), source).unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    for kind in [FlakeKind::WallClock, FlakeKind::FixedTimeout] {
        assert!(
            model.flake_sites.iter().any(|site| site.kind == kind),
            "a timer-only loop still depends on {kind:?}"
        );
    }
}

#[test]
fn a_deadline_break_in_a_real_do_while_body_is_also_a_wait_cap() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/m6-polls");
    let model = Model::build(&root, &BuildOptions::default());
    let deadline_clocks: Vec<_> = model
        .flake_sites
        .iter()
        .filter(|site| {
            site.file == "action-poll.ts"
                && site.kind == FlakeKind::WallClock
                && matches!(site.span.line, 19 | 29)
        })
        .map(|site| site.span.line)
        .collect();
    assert!(
        deadline_clocks.is_empty(),
        "deadline clocks: {deadline_clocks:?}"
    );
}

/// The draws of one source file, each with the code it is written in.
fn enclosings(name: &str, source: &str) -> Vec<(u32, Enclosing)> {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("helper.ts"), source).unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    model
        .flake_sites
        .iter()
        .filter(|site| site.kind == FlakeKind::UnseededRandom)
        .map(|site| (site.span.line, site.enclosing.clone()))
        .collect()
}

#[test]
fn each_site_records_the_innermost_named_code_it_is_written_in() {
    let named = |name: &str| Enclosing::Named(name.to_owned());
    assert_eq!(
        enclosings(
            "flake-enclosing",
            "import { beforeEach, vi } from 'vitest'\n\
             export const SEED = Math.random()\n\
             export class Provider {\n\
             \x20 count = Math.random()\n\
             \x20 constructor() { this.n = Math.random() }\n\
             \x20 search() { return [1].map(() => Math.random()) }\n\
             \x20 pick = () => Math.random()\n\
             }\n\
             export function draw() { return Math.random() }\n\
             export const roll = () => Math.random()\n\
             export const api = { get() { return Math.random() }, put: () => Math.random() }\n\
             export const handlers = [wrap(() => Math.random())]\n\
             beforeEach(() => { Math.random() })\n\
             vi.mock('./x', () => ({ v: Math.random() }))\n\
             exports.later = function () { return Math.random() }\n\
             export default function () { return Math.random() }\n\
             register(() => Math.random())\n",
        ),
        [
            (2, Enclosing::Module),
            (4, named("Provider")),
            (5, named("Provider")),
            (6, named("search")),
            (7, named("pick")),
            (9, named("draw")),
            (10, named("roll")),
            (11, named("get")),
            (11, named("put")),
            (12, named("handlers")),
            (13, Enclosing::Module),
            (14, Enclosing::Module),
            (15, named("later")),
            (16, named("default")),
            (17, Enclosing::Anonymous),
        ]
    );
}

#[test]
fn each_file_records_the_names_its_named_code_references() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("flake-names");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("helper.ts"),
        "export class Provider {\n\
         \x20 async search() { return Math.random() }\n\
         \x20 async getRandom() { return (await this.search())[0] }\n\
         \x20 getById(id: string) { return { id } }\n\
         }\n\
         export const make = () => new Provider()\n",
    )
    .unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    let names = model
        .flake_names
        .iter()
        .find(|n| n.file == "helper.ts")
        .unwrap();
    let of = |name: &str| -> Vec<&str> {
        names.calls[&Enclosing::Named(name.to_owned())]
            .iter()
            .map(String::as_str)
            .collect()
    };
    assert_eq!(of("getRandom"), ["search"]);
    assert_eq!(of("make"), ["Provider"]);
    assert_eq!(of("search"), ["Math", "random"]);
    assert_eq!(of("getById"), ["id"]);
    // A file with no tests keeps no offsets: only a test's own code is placed by them.
    assert!(names.uses.is_empty());
}

#[test]
fn names_follow_imports_renamed_destructuring_literal_keys_and_export_links() {
    let root = Path::new(env!("CARGO_TARGET_TMPDIR")).join("flake-names-links");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("helper.ts"),
        "import fixture, { makeId as id } from './mock'\n\
         import * as mock from './mock'\n\
         export function viaAlias() { return fixture() + id() }\n\
         export function viaPattern(p: object) { const { search: s } = p as never; return s }\n\
         export function viaLiteral(p: never) { return p['search']() }\n\
         export function viaKey(p: never, k: string) { return p[k]() }\n\
         export function viaValues() { return Object.values(mock) }\n\
         function local() { return 1 }\n\
         export { local as renamed }\n\
         export default viaAlias\n",
    )
    .unwrap();
    let model = Model::build(&root, &BuildOptions::default());
    let names = model
        .flake_names
        .iter()
        .find(|n| n.file == "helper.ts")
        .unwrap();
    let of = |name: &str| -> Vec<&str> {
        names.calls[&Enclosing::Named(name.to_owned())]
            .iter()
            .map(String::as_str)
            .collect()
    };
    assert_eq!(of("viaAlias"), ["default", "fixture", "id", "makeId"]);
    assert!(of("viaPattern").contains(&"search"));
    assert!(of("viaLiteral").contains(&"search"));
    assert!(of("viaKey").contains(&FlakeNames::DYNAMIC));
    assert!(of("viaValues").contains(&FlakeNames::DYNAMIC));
    assert_eq!(of("renamed"), ["local"]);
    assert_eq!(of("default"), ["viaAlias"]);
    // An export list or a default export of a binding runs nothing at module scope.
    let top = names.calls.get(&Enclosing::Module);
    assert!(top.is_none_or(|top| !top.contains("local") && !top.contains("viaAlias")));
}
