# Code Signing Policy

AgentWatch's Windows installers (NSIS `.exe` and MSI) are signed using a
certificate provided free of charge by [SignPath.io](https://signpath.io)
through the [SignPath Foundation](https://signpath.org) for open source
projects. This document describes how signing is governed, per SignPath
Foundation's requirements for participating projects.

## Roles

AgentWatch is currently maintained by a single maintainer, who holds all
three SignPath roles:

- **Author** — writes the code that is built and submitted for signing.
- **Reviewer** — reviews changes before a release is cut.
- **Approver** — approves each individual signing request in the SignPath
  dashboard before a binary is signed.

Both the GitHub account and the SignPath account used to manage this
project require multi-factor authentication, per SignPath Foundation's
eligibility conditions.

## What gets signed

Only binaries built by AgentWatch's own GitHub Actions workflow
(`.github/workflows/build-windows.yml`), from source in this repository,
are submitted for signing. No third-party binaries are signed under this
certificate.

## Privacy

AgentWatch is local-first: it only reads local files under
`~/.claude/sessions` and `~/.claude/projects` to display session status,
and never transmits this data anywhere. See the [README](README.md) for
details. Code signing does not change this — it only adds a cryptographic
signature to the installer binaries so Windows SmartScreen can verify their
publisher and integrity.

## Attribution

Code signing for this project is provided free of charge by the
[SignPath Foundation](https://signpath.org), using the
[SignPath.io](https://signpath.io) code signing platform.
