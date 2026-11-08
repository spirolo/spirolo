# Support

Spirolo reads JavaScript and TypeScript test suites and reports signals worth
checking. It is an advisory alpha. A finding needs review, and a clean report
does not establish that a test suite is correct.

## Where to ask

Start with [what Spirolo checks](docs/categories.md), [rule tiers](docs/tiers.md), and
[CI reports](docs/ci-reports.md). Run `spirolo explain <rule>` to read what a rule
checks and how it has been measured.

In this repository's **Issues** tab, choose **New issue**, then:

- **Support question** for help with commands, configuration, or interpreting output.
- **Bug report** for crashes, installation failures, or unexpected command behavior.
- **False positive** for a finding that is wrong about your tests.
- **Feature request** for behavior Spirolo does not support yet.

Report vulnerabilities privately through [SECURITY.md](SECURITY.md). For conduct
concerns, use the private channel in the [code of conduct](CODE_OF_CONDUCT.md).

A maintainer gives every new issue a first answer within 7 days. Fixes have no deadline.

## What to include

Include `spirolo --version`, your operating system, the command you ran, what you
expected, and what happened. For a finding, include its rule name and the relevant
output. A small project or code excerpt that reproduces the problem helps more
than a full log. Remove secrets, private paths, and code you cannot share before
posting.

## What to expect

Support happens in issues as maintainers and contributors have time. Beyond the
first answer, there is no guaranteed response time or fix date. A reply may ask
for a smaller reproduction, explain a limitation, suggest a configuration change,
or identify work needed for a fix. An acknowledged report is not a commitment to
implement a change.

To contribute a fix or documentation, see [CONTRIBUTING.md](CONTRIBUTING.md).
