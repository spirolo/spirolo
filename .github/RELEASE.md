# Tag releases

Push a tag matching the Cargo/npm version, such as `v0.1.0-alpha.1`.
The workflow builds Linux and macOS
for x64 and ARM64 with the locked Cargo dependencies. It stages the five npm
tarballs, native archives and binaries, and `SHA256SUMS` as the `release` Actions
artifact. The npm dry run uses `--provenance --ignore-scripts --access public`.
Prereleases use their identifier (`alpha`, `beta`, `rc`); stable versions use
`latest`.

Publication is disabled by default. Setting the repository variable
`RELEASE_PUBLISH=true` enables the publish job **only in a public repository**.
That job verifies checksums again, publishes native packages before the launcher,
and creates a GitHub release with all staged files and checksums attached.
Prerelease tags create prereleases. The package job still performs its dry run
before the publish job starts. No workflow dispatch or pull request can publish.

After the publish job, the `registry` job lists the version on the
[MCP Registry](https://registry.modelcontextprotocol.io) as
`io.github.spirolo/spirolo`, from the generated `server.json`. It signs in with
the workflow's own identity (no token), using a pinned `mcp-publisher` whose
signature is verified first. The registry accepts a version only when that npm
version's `package.json` carries the same `mcpName`, which is why the job runs
after npm. It is a separate job: if the registry is down, rerun that job alone.
A registry entry cannot be edited once published; a fix is a new version.

Before preparing a release, maintainers need:

- After creating the repo, run `scripts/public/settings.sh spirolo/spirolo --apply`.
- A public GitHub repository with Actions enabled and permission to push tags
  and manage repository variables.
- Access to the npm scope and all five package settings. Create any missing
  packages before configuring trusted publishing.
- Each package's `repository` set to the public repository. Keep the metadata
  generator in `npm/release.mjs` consistent with the package manifests.
- An npm trusted publisher for each package, configured with the GitHub owner,
  repository name and workflow filename `release.yml`. The workflow uses no
  GitHub environment. Node 24 supplies npm with trusted publishing support;
  the publish job grants `id-token: write` and uses `npm publish --provenance`.

To rehearse and publish a release:

1. Set the workspace package version in `Cargo.toml`, run
   `cargo update --workspace` to record the new version of every workspace
   crate in `Cargo.lock`, run `node npm/release.mjs sync`, and update
   `CHANGELOG.md`. Commit `Cargo.lock` with the release metadata: the release
   workflow builds with `cargo build --locked`, which fails when the lockfile
   still records the old version. Run `just release-check` and `actionlint`
   before tagging; `node npm/release.mjs check` fails while `Cargo.lock` and
   the workspace version disagree.
2. Leave `RELEASE_PUBLISH` unset for the first run. Create an annotated tag
   matching the version (`git tag -a v<version> -m 'Release <version>'`) and
   push it to the repository (`git push origin v<version>`).
3. Wait for the Release workflow's build and package jobs to pass. Download
   the `release` artifact from that run, extract it, and run
   `sha256sum --check SHA256SUMS` in the extracted directory. Inspect the npm
   tarballs for their native payloads and test installation on each supported
   platform. A successful npm dry run does not prove registry authorization
   or provenance generation.
4. Set the repository variable `RELEASE_PUBLISH` to `true`, then rerun all jobs
   of the successful tag workflow. The publish job verifies checksums,
   publishes the packages and creates the GitHub release. Verify npm provenance,
   registry installation and the release attachments after publication.
5. Remove `RELEASE_PUBLISH` to return subsequent tags to dry runs. npm package
   versions cannot be republished; use a new version and tag for later releases.

References: [npm provenance](https://docs.npmjs.com/generating-provenance-statements/)
and [npm trusted publishing](https://docs.npmjs.com/trusted-publishers/).
