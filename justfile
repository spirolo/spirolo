# The snapshot's standalone gate. Requires Rust, cargo-nextest, Node >=20.10, npm and Git.
default:
    @just --list

# Build the shipped workspace, using its committed lockfile.
build:
    cargo build --locked --workspace

# Every shipped Rust test, plus doctests (nextest does not run doctests).
test:
    cargo nextest run --locked --workspace --all-features --profile ci
    cargo test --locked --workspace --all-features --doc

# Discover every committed Node suite, including new suites added later.
# The workflow suite reads YAML with the parser locked in .github/package-lock.json.
node-tests:
    npm ci --prefix .github --ignore-scripts --no-audit --no-fund
    node scripts/public/node-tests.mjs

# Release metadata, launcher and workflow contracts (no release build or registry needed).
release-check:
    npm ci --prefix .github --ignore-scripts --no-audit --no-fund
    node --test .github/workflows.test.mjs npm/launcher.test.mjs npm/release.test.mjs npm/publish.test.mjs

# Install the real staged tarballs and verify each native package, after `node npm/stage.mjs target/rc`.
rc-test:
    node --test npm/staged.test.mjs

ci: build test node-tests
