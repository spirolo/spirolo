//! Follow named package scripts from CI. This reads commands; it never runs them.

use std::collections::BTreeSet;

use crate::manifest::Manifest;
use crate::{Gate, GateTool};

/// Keep the original CI commands and append commands reached through package scripts.
pub fn expand_steps(root: &std::path::Path, manifests: &[Manifest], gates: &mut [Gate]) {
    let Some(manifest) = manifests.iter().find(|m| m.dir.is_empty()) else {
        return;
    };
    // Files are kept inside the root by comparing canonical paths, so the root is made
    // canonical too: a root reached through a symlink (`/var` on macOS) still holds its files.
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    for gate in gates.iter_mut().filter(|gate| gate.tool == GateTool::Ci) {
        let mut expander = Expander {
            root: &root,
            manifests,
            stack: Vec::new(),
            seen: BTreeSet::new(),
            out: Vec::new(),
        };
        for step in &gate.steps {
            expander.expand(step, manifest);
        }
        gate.steps.extend(expander.out);
    }
}

/// The words one shell command runs, past assignments and process wrappers.
///
/// Skips leading `NAME=value` words and the wrappers CI puts in front of a test command:
/// `xvfb-run -a`, `timeout 20m`, `cross-env`, `env`, `dotenv --`, `nice`, `time`, `sudo`
/// and `nohup`.
pub fn command_words(command: &str) -> Vec<&str> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let mut start = 0;
    loop {
        while words
            .get(start)
            .is_some_and(|word| word.contains('=') && !word.starts_with('-'))
        {
            start += 1;
        }
        // Each wrapper with the short flags that take a separate value.
        let valued: &[&str] = match words.get(start).copied() {
            Some("xvfb-run") => &["-e", "-f", "-n", "-p", "-s"],
            Some("timeout") => &["-k", "-s"],
            Some("nice") => &["-n"],
            Some("sudo") => &["-g", "-u"],
            Some("time") => &["-f", "-o"],
            Some("dotenv") => &["-c", "-e", "-v"],
            Some("env" | "cross-env" | "nohup") => &[],
            _ => break,
        };
        let timeout = words[start] == "timeout";
        start += 1;
        while let Some(word) = words.get(start) {
            if *word == "--" {
                start += 1;
                break;
            }
            if !word.starts_with('-') {
                break;
            }
            start += if valued.contains(word) { 2 } else { 1 };
        }
        if timeout {
            // The duration.
            start += 1;
        }
    }
    words.get(start..).unwrap_or_default().to_vec()
}

/// One CI gate's walk through scripts, recipes and Node wrappers.
struct Expander<'a> {
    root: &'a std::path::Path,
    manifests: &'a [Manifest],
    stack: Vec<String>,
    /// Package scripts already followed for this gate. A workspace fan-out reaches the same
    /// script from many places; following it once keeps the walk linear.
    seen: BTreeSet<String>,
    out: Vec<String>,
}

impl<'a> Expander<'a> {
    fn expand(&mut self, command: &str, manifest: &'a Manifest) {
        if self.stack.len() >= 16 {
            return;
        }
        let mut manifest = manifest;
        let mut directory = manifest.dir.clone();
        for part in command.split(['\n', ';', '&', '|']) {
            let words = command_words(part);
            if words.first() == Some(&"cd") {
                let Some(next) = words
                    .get(1)
                    .filter(|path| !path.contains('$'))
                    .and_then(|path| {
                        crate::paths::join_relative(&directory, path.trim_matches(['\'', '"']))
                    })
                else {
                    return;
                };
                // Package managers run the scripts of the nearest enclosing `package.json`.
                let Some(next_manifest) = self
                    .manifests
                    .iter()
                    .filter(|m| {
                        m.dir.is_empty()
                            || next == m.dir
                            || next
                                .strip_prefix(&m.dir)
                                .is_some_and(|r| r.starts_with('/'))
                    })
                    .max_by_key(|m| m.dir.len())
                else {
                    return;
                };
                directory = next;
                manifest = next_manifest;
                continue;
            }
            let Some(program) = words.first() else {
                continue;
            };
            match *program {
                "yarn" if words.get(1) == Some(&"workspace") => {
                    if let Some(workspace) = words.get(2).and_then(|name| {
                        self.manifests
                            .iter()
                            .find(|m| m.name.as_deref() == Some(*name))
                    }) {
                        let index = if words.get(3) == Some(&"run") { 4 } else { 3 };
                        if let Some(name) = words.get(index) {
                            self.follow(name, workspace, "");
                        }
                    }
                }
                "yarn" if words.get(1) == Some(&"workspaces") => {
                    // `yarn workspaces run x` (v1), `yarn workspaces foreach [-A …] run x`.
                    if let Some(index) = words.iter().position(|word| *word == "run") {
                        if let Some(name) = words.get(index + 1) {
                            self.fan_out(&[*name], "");
                        }
                    }
                }
                "npm" | "pnpm" | "yarn" => self.package_manager(&words, manifest, &directory),
                "npx" | "bunx" => {
                    let rest: Vec<&str> = words[1..]
                        .iter()
                        .copied()
                        .skip_while(|word| word.starts_with('-'))
                        .collect();
                    self.task_runner(&rest);
                }
                "turbo" | "lerna" | "nx" => self.task_runner(&words),
                "npm-run-all" | "run-s" | "run-p" => {
                    for pattern in &words[1..] {
                        let pattern = pattern.trim_matches(['\'', '"']);
                        if pattern.starts_with('-') {
                            continue;
                        }
                        let Ok(glob) = globset::Glob::new(pattern) else {
                            continue;
                        };
                        let matcher = glob.compile_matcher();
                        for name in manifest
                            .named_scripts
                            .keys()
                            .filter(|name| matcher.is_match(name))
                        {
                            self.follow(name, manifest, "");
                        }
                    }
                }
                "make" => self.make(&words[1..], manifest, &directory),
                _ => {
                    let script = if *program == "node" {
                        words.get(1).copied()
                    } else {
                        Some(*program)
                    };
                    self.node(script, &directory);
                }
            }
        }
    }

    /// `npm`/`pnpm`/`yarn` running a script here, in a named or located workspace, or in
    /// every workspace (`-r`, `--workspaces`). A selector the reader cannot match falls back
    /// to every workspace that defines the script, so an unread filter never hides one.
    fn package_manager(&mut self, words: &[&str], manifest: &'a Manifest, directory: &str) {
        let program = words[0];
        let mut targets: Option<Vec<&'a Manifest>> = None;
        let mut everywhere = false;
        let mut rest = &words[1..];
        while let Some(word) = rest.first().filter(|word| word.starts_with('-')) {
            let (flag, inline) = word
                .split_once('=')
                .map_or((*word, None), |(flag, value)| (flag, Some(value)));
            let valued = matches!(
                flag,
                "--dir" | "--prefix" | "--cwd" | "-C" | "--filter" | "-F"
            ) || (program == "npm" && matches!(flag, "-w" | "--workspace"));
            let value = if valued {
                inline.or_else(|| rest.get(1).copied())
            } else {
                None
            };
            let consumed = if valued && inline.is_none() { 2 } else { 1 };
            rest = rest.get(consumed..).unwrap_or_default();
            let Some(value) = value.map(|value| value.trim_matches(['\'', '"'])) else {
                everywhere |= matches!(flag, "-r" | "--recursive" | "--workspaces" | "-ws");
                continue;
            };
            let selected = targets.get_or_insert_with(Vec::new);
            match flag {
                "--dir" | "--prefix" | "--cwd" | "-C" => {
                    if let Some(found) = crate::paths::join_relative(directory, value)
                        .and_then(|dir| self.manifests.iter().find(|m| m.dir == dir))
                    {
                        selected.push(found);
                    }
                }
                _ => match self.selected(value) {
                    Some(found) => selected.extend(found),
                    None => everywhere = true,
                },
            }
        }
        everywhere |= rest
            .iter()
            .any(|word| matches!(*word, "--recursive" | "--workspaces"));
        if matches!(rest.first(), Some(&("run" | "run-script"))) {
            rest = &rest[1..];
        }
        let Some(name) = rest.first().copied() else {
            return;
        };
        // The arguments the script runs with: npm keeps flags before `--` for itself, pnpm
        // and yarn pass everything after the name.
        let after = &rest[1..];
        if program == "npm" {
            // `npm run x --prefix apps/web`: read npm's location flags as if they came first.
            let own = after.split(|word| *word == "--").next().unwrap_or_default();
            let mut located = vec![program];
            let mut kept = vec![name];
            let mut words = own.iter();
            while let Some(word) = words.next() {
                if matches!(*word, "--prefix" | "-w" | "--workspace") {
                    located.push(word);
                    located.extend(words.next());
                } else if word.starts_with("--prefix=") || word.starts_with("--workspace=") {
                    located.push(word);
                } else {
                    kept.push(word);
                }
            }
            if located.len() > 1 {
                located.extend(kept);
                located.extend_from_slice(&after[own.len()..]);
                self.package_manager(&located, manifest, directory);
                return;
            }
        }
        let args = if program == "npm" {
            after
                .iter()
                .position(|word| *word == "--")
                .map_or(&[][..], |at| &after[at + 1..])
        } else {
            after.strip_prefix(&["--"]).unwrap_or(after)
        }
        .join(" ");
        if everywhere {
            self.fan_out(&[name], &args);
        } else if let Some(targets) = targets {
            for target in targets {
                self.follow(name, target, &args);
            }
        } else if manifest.named_scripts.contains_key(name) {
            self.follow(name, manifest, &args);
        } else if matches!(name, "exec" | "dlx" | "turbo" | "lerna" | "nx") {
            let at = usize::from(name != "turbo" && name != "lerna" && name != "nx");
            self.task_runner(&rest[at..]);
        }
    }

    /// The workspaces a `--filter`/`--workspace` value names, by package name (globs allowed)
    /// or by directory (`./apps/web`, `{apps/web}`, `apps/*`). `None` when nothing matches.
    fn selected(&self, value: &str) -> Option<Vec<&'a Manifest>> {
        let pattern = value
            .trim_start_matches('!')
            .trim_start_matches("...")
            .trim_end_matches("...")
            .trim_end_matches('^')
            .trim_start_matches('{')
            .trim_end_matches('}');
        let by_dir = pattern.strip_prefix("./").unwrap_or(pattern);
        let name = globset::Glob::new(pattern).ok()?.compile_matcher();
        let dir = globset::Glob::new(by_dir.trim_end_matches('/'))
            .ok()?
            .compile_matcher();
        let found: Vec<&Manifest> = self
            .manifests
            .iter()
            .filter(|m| {
                m.name.as_deref().is_some_and(|n| name.is_match(n))
                    || (!m.dir.is_empty() && dir.is_match(&m.dir))
            })
            .collect();
        (!found.is_empty()).then_some(found)
    }

    /// `turbo run a b`, `lerna run a`, `nx run-many -t a`, `nx run app:a`: each task in every
    /// workspace that defines it. Filters are not read, so the walk over-approximates.
    fn task_runner(&mut self, words: &[&str]) {
        let tasks: Vec<&str> = match words {
            ["turbo", rest @ ..] => {
                let rest = if rest.first() == Some(&"run") {
                    &rest[1..]
                } else {
                    rest
                };
                rest.iter()
                    .copied()
                    .take_while(|word| !word.starts_with('-'))
                    .collect()
            }
            ["lerna", "run", task, ..] => vec![*task],
            ["nx", "run", target, ..] => target
                .rsplit_once(':')
                .map(|(_, task)| task)
                .into_iter()
                .collect(),
            ["nx", "run-many" | "affected", rest @ ..] => {
                let mut tasks = Vec::new();
                let mut iter = rest.iter();
                while let Some(word) = iter.next() {
                    let value = match word.split_once('=') {
                        Some(("--target" | "--targets" | "-t", value)) => Some(value),
                        None if matches!(*word, "--target" | "--targets" | "-t") => {
                            iter.next().copied()
                        }
                        _ => None,
                    };
                    tasks.extend(value.into_iter().flat_map(|value| value.split(',')));
                }
                tasks
            }
            _ => Vec::new(),
        };
        self.fan_out(&tasks, "");
    }

    /// Follow each named script in every workspace that defines it.
    fn fan_out(&mut self, names: &[&str], args: &str) {
        let manifests = self.manifests;
        for name in names {
            for manifest in manifests {
                if manifest.named_scripts.contains_key(*name) {
                    self.follow(name, manifest, args);
                }
            }
        }
    }

    /// Run a package script with the arguments forwarded to it, appended to its command as
    /// package managers do.
    fn follow(&mut self, name: &str, manifest: &'a Manifest, args: &str) {
        let key = format!("{}:{name} {args}", manifest.dir);
        if self.stack.contains(&key) || !self.seen.insert(key.clone()) {
            return;
        }
        let Some(script) = manifest.named_scripts.get(name) else {
            return;
        };
        let command = if args.is_empty() {
            script.clone()
        } else {
            format!("{script} {args}")
        };
        if manifest.dir.is_empty() {
            self.out.push(command.clone());
        } else {
            self.out.push(format!("cd {} && {command}", manifest.dir));
        }
        self.stack.push(key);
        self.expand(&command, manifest);
        self.stack.pop();
    }

    /// `make` runs the targets of the Makefile in its directory: where the step stands, or
    /// the one `-C`/`--directory` names. Recipes run in that directory.
    fn make(&mut self, words: &[&str], manifest: &'a Manifest, directory: &str) {
        let mut directory = Some(directory.to_owned());
        let mut targets = Vec::new();
        let mut rest = words.iter();
        while let Some(word) = rest.next() {
            let named = match *word {
                "-C" | "--directory" => rest.next().copied(),
                _ => word
                    .strip_prefix("--directory=")
                    .or_else(|| word.strip_prefix("-C").filter(|dir| !dir.is_empty())),
            };
            if let Some(named) = named {
                directory = directory.and_then(|directory| {
                    crate::paths::join_relative(&directory, named.trim_matches(['\'', '"']))
                });
            } else if !word.starts_with('-') && !word.contains('=') {
                targets.push(*word);
            }
        }
        let Some(directory) = directory.filter(|directory| !directory.contains('$')) else {
            return;
        };
        let makefile = self.root.join(&directory).join("Makefile");
        if !makefile
            .canonicalize()
            .is_ok_and(|path| path.starts_with(self.root))
        {
            return;
        }
        let Ok(source) = std::fs::read_to_string(makefile) else {
            return;
        };
        let root = self.manifests.iter().find(|m| m.dir.is_empty());
        for target in targets {
            let key = format!("make:{directory}:{target}");
            if self.stack.contains(&key) {
                continue;
            }
            self.stack.push(key);
            let mut selected = false;
            for line in source.lines() {
                if line.starts_with('\t') {
                    if selected {
                        let recipe = line.trim_start().trim_start_matches('@');
                        if directory.is_empty() {
                            self.out.push(recipe.to_owned());
                            self.expand(recipe, manifest);
                        } else {
                            let recipe = format!("cd {directory} && {recipe}");
                            self.out.push(recipe.clone());
                            self.expand(&recipe, root.unwrap_or(manifest));
                        }
                    }
                } else if !line.trim().is_empty() && !line.starts_with('#') {
                    selected = line.split_once(':').is_some_and(|(names, _)| {
                        names.split_whitespace().any(|name| name == target)
                    });
                }
            }
            self.stack.pop();
        }
    }

    fn node(&mut self, script: Option<&str>, directory: &str) {
        if let Some(path) = script
            .filter(|path| {
                std::path::Path::new(path).extension().is_some_and(|ext| {
                    ["js", "mjs", "cjs"]
                        .iter()
                        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
                })
            })
            .and_then(|path| crate::paths::join_relative(directory, path))
        {
            let key = format!("node:{path}");
            if self.stack.contains(&key) {
                return;
            }
            let absolute = self.root.join(&path);
            if !absolute
                .canonicalize()
                .is_ok_and(|path| path.starts_with(self.root))
            {
                return;
            }
            if let Ok(source) = std::fs::read_to_string(absolute) {
                self.stack.push(key);
                for (dir, command) in super::node::spawned(&source, &path, directory) {
                    self.out.push(format!("cd '{dir}' && {command}"));
                    if let Some(manifest) = self.manifests.iter().find(|m| m.dir == dir) {
                        self.expand(&command, manifest);
                    }
                }
                self.stack.pop();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The commands one CI step reaches over the given `(path, package.json)` manifests.
    fn reached(manifests: &[(&str, &str)], step: &str) -> Vec<String> {
        let manifests: Vec<Manifest> = manifests
            .iter()
            .map(|(path, source)| crate::manifest::read(path, source).unwrap())
            .collect();
        let mut gate = Gate::blank(".github/workflows/ci.yml", GateTool::Ci);
        gate.steps.push(step.to_owned());
        let mut gates = [gate];
        expand_steps(std::path::Path::new("/nonexistent"), &manifests, &mut gates);
        let [gate] = gates;
        gate.steps[1..].to_vec()
    }

    /// The commands one CI step reaches through the Makefiles written under a scratch root.
    fn reached_through_make(step: &str) -> Vec<String> {
        let root = std::env::temp_dir().join(format!("spirolo-make-{}", std::process::id()));
        std::fs::create_dir_all(root.join("e2e")).unwrap();
        std::fs::write(root.join("Makefile"), "test:\n\tnpm run lint\n").unwrap();
        std::fs::write(root.join("e2e/Makefile"), "test:\n\tnpx playwright test\n").unwrap();
        let manifests = [crate::manifest::read("package.json", ROOT.1).unwrap()];
        let mut gate = Gate::blank(".github/workflows/ci.yml", GateTool::Ci);
        gate.steps.push(step.to_owned());
        let mut gates = [gate];
        expand_steps(&root, &manifests, &mut gates);
        std::fs::remove_dir_all(&root).unwrap();
        let [gate] = gates;
        gate.steps[1..].to_vec()
    }

    const ROOT: (&str, &str) = ("package.json", r#"{"name": "root", "scripts": {}}"#);
    const WEB: (&str, &str) = (
        "apps/web/package.json",
        r#"{"name": "web", "scripts": {"test:e2e": "playwright test"}}"#,
    );
    const DOCS: (&str, &str) = (
        "apps/docs/package.json",
        r#"{"name": "@site/docs", "scripts": {"test:e2e": "playwright test -c e2e"}}"#,
    );

    #[test]
    fn command_words_skip_assignments_and_process_wrappers() {
        assert_eq!(
            command_words("CI=1 xvfb-run -a timeout --signal=INT -k 2m 105m pnpm test:e2e"),
            ["pnpm", "test:e2e"]
        );
        assert_eq!(
            command_words("xvfb-run --auto-servernum npm run x"),
            ["npm", "run", "x"]
        );
        assert_eq!(command_words("cross-env NODE_ENV=test jest"), ["jest"]);
        assert_eq!(
            command_words("dotenv -e .env.ci -- yarn e2e"),
            ["yarn", "e2e"]
        );
    }

    #[test]
    fn a_wrapped_script_run_is_followed() {
        let root = (
            "package.json",
            r#"{"scripts": {"test:e2e": "playwright test"}}"#,
        );
        let steps = reached(&[root], "xvfb-run -a timeout 20m pnpm test:e2e");
        assert_eq!(steps, ["playwright test"]);
    }

    #[test]
    fn arguments_after_the_script_name_reach_its_command() {
        let root = (
            "package.json",
            r#"{"scripts": {"e2e": "playwright test", "test:e2e": "pnpm e2e"}}"#,
        );
        let forwarded = ["playwright test --config apps/web/playwright.config.ts"];
        for step in [
            "npm run e2e -- --config apps/web/playwright.config.ts",
            "pnpm e2e --config apps/web/playwright.config.ts",
            "yarn run e2e -- --config apps/web/playwright.config.ts",
        ] {
            let steps = reached(&[root], step);
            assert_eq!(steps, forwarded, "{step}");
        }
        // npm keeps flags before `--` for itself; a nested run passes the arguments on.
        assert_eq!(
            reached(&[root], "npm run e2e --silent"),
            ["playwright test"]
        );
        let steps = reached(&[root], "pnpm test:e2e -c e2e");
        assert_eq!(steps, ["pnpm e2e -c e2e", "playwright test -c e2e"]);
    }

    #[test]
    fn make_reads_the_makefile_of_the_directory_it_runs_in() {
        assert_eq!(reached_through_make("make test"), ["npm run lint"]);
        for step in [
            "make -C e2e test",
            "make --directory=e2e test",
            "cd e2e && make test",
        ] {
            assert_eq!(
                reached_through_make(step),
                ["cd e2e && npx playwright test"],
                "{step}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn make_does_not_read_a_makefile_outside_the_root() {
        let scratch = std::env::temp_dir().join(format!("spirolo-make-out-{}", std::process::id()));
        let (root, outside) = (scratch.join("repo"), scratch.join("outside"));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("Makefile"), "test:\n\tnpx playwright test\n").unwrap();
        std::os::unix::fs::symlink(&outside, root.join("linked")).unwrap();
        let manifests = [crate::manifest::read("package.json", ROOT.1).unwrap()];
        let mut gate = Gate::blank(".github/workflows/ci.yml", GateTool::Ci);
        gate.steps.push("make -C linked test".to_owned());
        let mut gates = [gate];
        expand_steps(&root, &manifests, &mut gates);
        std::fs::remove_dir_all(&scratch).unwrap();
        assert!(gates[0].steps[1..].is_empty(), "{:?}", gates[0].steps);
    }

    #[test]
    fn a_cd_without_a_manifest_runs_scripts_of_the_nearest_package() {
        let steps = reached(
            &[ROOT, WEB],
            "cd apps/web/e2e && pnpm test:e2e && pnpm lint",
        );
        assert_eq!(steps, ["cd apps/web && playwright test"]);
    }

    #[test]
    fn a_filtered_script_run_follows_the_named_workspace_only() {
        let steps = reached(&[ROOT, WEB, DOCS], "pnpm --filter web test:e2e");
        assert_eq!(steps, ["cd apps/web && playwright test"]);
        let steps = reached(&[ROOT, WEB, DOCS], "pnpm -F './apps/docs' run test:e2e");
        assert_eq!(steps, ["cd apps/docs && playwright test -c e2e"]);
        let steps = reached(&[ROOT, WEB, DOCS], "npm --workspace=@site/* run test:e2e");
        assert_eq!(steps, ["cd apps/docs && playwright test -c e2e"]);
    }

    #[test]
    fn npm_reads_a_location_given_after_the_script_name() {
        for step in [
            "npm run test:e2e --prefix apps/web",
            "npm run test:e2e --prefix=apps/web",
            "npm run test:e2e -w web",
            "npm run test:e2e --workspace=web",
        ] {
            let steps = reached(&[ROOT, WEB, DOCS], step);
            assert_eq!(steps, ["cd apps/web && playwright test"], "{step}");
        }
    }

    #[test]
    fn an_unmatched_filter_follows_every_workspace_with_the_script() {
        let steps = reached(&[ROOT, WEB, DOCS], "pnpm --filter $PROJECT test:e2e");
        assert_eq!(steps.len(), 2, "{steps:?}");
    }

    #[test]
    fn recursive_and_task_runner_fan_outs_follow_every_workspace() {
        for step in [
            "pnpm -r test:e2e",
            "npm run test:e2e --workspaces",
            "npx turbo run test:e2e --filter=web",
            "pnpm turbo test:e2e",
            "yarn workspaces foreach -A run test:e2e",
            "lerna run test:e2e",
            "nx run-many -t lint,test:e2e",
        ] {
            let steps = reached(&[ROOT, WEB, DOCS], step);
            assert!(
                steps.contains(&"cd apps/web && playwright test".to_owned()),
                "{step}: {steps:?}"
            );
        }
    }
}
