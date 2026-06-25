# Release Strategy

## Goals
- Keep releases predictable and auditable.
- Preserve a clear version history.
- Make signed artifacts and changelogs part of the normal release path.

## Proposed flow
- Merge changes through pull requests.
- Cut tagged releases from the main branch.
- Generate changelog entries from commit history or release notes.
- Publish platform artifacts and update distribution channels.

## Principles
- Semantic versioning should drive compatibility expectations.
- Release automation should be boring and repeatable.
- Security-sensitive changes should not be released without review.
