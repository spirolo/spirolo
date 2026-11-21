# The snapshot's standalone gate. Requires Rust, cargo-nextest, typos, cargo-deny, Node >=20.10, npm and Git.
default:
    @just --list

# Formatting, without rewriting anything (`cargo fmt --all` fixes it).
fmt-check:
    cargo fmt --all --check

# Spelling, as configured in typos.toml.
typos:
    typos

# Clippy over every target and feature; a warning is an error.
clippy:
    cargo clippy --locked --workspace --all-targets --all-features -- -D warnings

# Advisories, licences, bans and sources, as configured in deny.toml.
deny:
    cargo deny check

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
    node --test .github/workflows.test.mjs npm/launcher.test.mjs npm/release.test.mjs npm/claims.test.mjs npm/publish.test.mjs npm/release-notes.test.mjs

# Install the real staged tarballs and verify each native package, after `node npm/stage.mjs target/rc`.
rc-test:
    node --test npm/staged.test.mjs

# The website: public check, type check, static build (`website/out/`). Needs pnpm.
website:
    cd website && pnpm install --frozen-lockfile && pnpm check:public && pnpm types:check && pnpm build

ci: fmt-check typos clippy build test node-tests deny
