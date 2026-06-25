# GitHub Actions Strategy

## Goals
- Keep validation fast for pull requests.
- Separate unit, integration, fuzz, benchmark, and documentation jobs.
- Make release automation explicit and predictable.

## Workflow layout
- `ci.yml` for formatting, linting, and fast tests.
- `integration.yml` for heavier end-to-end validation.
- `fuzz.yml` for targeted fuzz execution.
- `bench.yml` for performance regression tracking.
- `docs.yml` for book and docs validation.
- `release.yml` for tagging and publication steps.

## Principles
- Small workflows are easier to rerun and debug.
- Heavy jobs should be gated or scheduled so pull requests stay responsive.
- Secrets should be used sparingly and only where release or signing requires them.
