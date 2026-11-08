# Security policy

## Supported versions

Spirolo is in its advisory alpha. Only the latest `0.1.0-alpha` release receives fixes; a fix
ships as the next alpha, not as a patch to an older one.

| Version | Supported |
| --- | --- |
| `0.1.0-alpha.*` (latest) | yes |
| `0.1.0-rc.1` and older | no (never published) |

This covers the `spirolo` binary, its npm launcher and platform packages, and the
VS Code extension built from this repository.

## Reporting a vulnerability

Report it privately through GitHub: open the repository's **Security** tab and choose
**Report a vulnerability** (GitHub private vulnerability reporting). Please do not open a
public issue, pull request or discussion for it.

Include the version (`spirolo --version`), the platform, the command you ran, and a minimal
project or input that reproduces it. The discussion stays in the private advisory until a
fixed release is out.

A maintainer answers a report within 7 days. A fix has no fixed deadline; the advisory
says when a fixed release is expected once the cause is known.

Analysis reads the files of the project you point it at. It does not run tests, shell out or
access the network during analysis, so a project's files making `spirolo` do any of those is in
scope.
