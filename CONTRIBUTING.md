# Contributing to edgefirst-v4l2

Thank you for contributing. This project follows the Au-Zone Software Process Standard (SPS); the canonical rules live in [EdgeFirstAI/.github](https://github.com/EdgeFirstAI/.github/blob/main/.github/copilot-instructions.md).

## Workflow

1. Branch from `main`: `feature/EDGEAI-###-short-description` or `bugfix/EDGEAI-###-short-description`.
2. Commit with the JIRA key and a DCO sign-off: `git commit -s -m "EDGEAI-###: Brief description"`.
3. Open a **draft** pull request to `main`. The Quick CI tier runs on every push; mark the PR ready once `ci-gate` is green.
4. Add the `ci:full` label for the full platform matrix, or `ci:hardware` for the on-target board lane, when the change touches device behaviour.

## Before pushing

```bash
make format
make lint
make test
```

## Code expectations

- Every `#[repr(C)]` struct added to `uapi` carries a compile-time size assertion for 64-bit Linux.
- Every public item has rustdoc.
- Device-backed tests skip cleanly when the device is absent.
- New dependencies must pass the license policy (`make sbom`) and be listed in NOTICE when they require attribution.

## Code of Conduct

This project follows the [Contributor Covenant](CODE_OF_CONDUCT.md).
