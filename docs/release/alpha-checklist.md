# Prerelease publication checklist

Use this checklist after [building and staging](../release.md) a release. It
separates local verification from publication. Package metadata and workflow files
in the release checkout determine the names, targets and destinations.

## Confirm the release identity

- [ ] Confirm the product name, package scope and repository in generated metadata.
- [ ] Confirm that the publishing account has access to each intended destination.
- [ ] Check that documentation links point to files available in the release repository.
- [ ] Read the release workflow before creating a tag: a tag can trigger publication.

## Verify the release commit

- [ ] Use a clean checkout of the intended commit.
- [ ] Set the version in `Cargo.toml`, run `node npm/release.mjs sync`, and review its diff.
- [ ] Run `node npm/release.mjs check` and the repository's required checks.
- [ ] Build all configured native targets and package the extension.
- [ ] Stage into a clean directory, run `just rc-test`, and verify `SHA256SUMS`.
- [ ] Check `artifacts.json` against the target list in `dist-workspace.toml`.

## Test installation

- [ ] Install the launcher and matching platform tarballs together in an empty project.
- [ ] Check `spirolo --version` against the staged package version.
- [ ] Run `spirolo check` on a real project and inspect diagnostics and findings.
- [ ] Verify the default view against `spirolo rule-status` and the [tier policy](../tiers.md).
- [ ] Repeat on the supported operating systems and architectures; record any untested target.
- [ ] Rehearse registry installation locally if needed, using the release guide.

A build that succeeds is not evidence that its binary runs on another platform.
A clean analysis result is not evidence of rule accuracy. Record what was actually
checked, including the project revision and configuration.

## Publish the verified artifacts

- [ ] Confirm the repository, registry, package names and prerelease tag again.
- [ ] Use the release workflow or the destination's publication procedure for the
      verified artifact set. Publish native packages before a launcher that depends on them.
- [ ] Check the resulting registry tags; do not assume a prerelease leaves the default
      installation unchanged.
- [ ] Download the published artifacts and verify their checksums.
- [ ] Install from the published destination in a clean project and check the version.
- [ ] Record the release URL, artifact checksums and platform smoke-test results.

Extension marketplace publication is separate from creating a VSIX or a repository
release. Confirm the extension's version and publisher before that step.
