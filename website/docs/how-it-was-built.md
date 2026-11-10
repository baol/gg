---
title: How it was built
description: Git Good is written by an AI coding agent under its maintainer's direction, against written specifications and tests.
---

# How it was built

Most of Git Good's code is written by an AI coding agent, Claude Code, working under the direction of its maintainer. The maintainer decides what the app does, writes and approves the requirements, tries every change on real repositories, and merges it. The agent writes the code, the tests and most of the documentation against those requirements. Nothing ships because it compiled: a change ships when its scenarios pass on six platforms and it works on the case it was made for.

This page says how that is kept honest. Everything it names is in the [repository](https://github.com/baol/gg).

## Written down first

Each behavior is written before it is built, in one owning document:

- **[Acceptance scenarios](https://github.com/baol/gg/blob/main/docs/acceptance.md)**: 697 numbered Given / When / Then scenarios, each naming the tests that cover it.
- **[Domain language](https://github.com/baol/gg/blob/main/docs/domain.md)**: one term for each thing the app shows, used the same way in the app, the specifications, this site and the CLI. That is why you read *History tree*, *Local Changes* and *Recovery* everywhere, and never two words for one thing.
- **[Architecture](https://github.com/baol/gg/blob/main/docs/architecture.md)**: system boundaries and contracts, such as the one place every Git process starts.
- **[Verification](https://github.com/baol/gg/blob/main/docs/verification.md)**: measured results, and checks that are still open.

A change that alters behavior updates its specification in the same commit.

## Tested where it runs

- **1,067 tests**, many of them driving the real window offscreen: they press keys, click buttons by their accessible names and read what is on screen.
- **Six platforms on every change**: Linux, macOS and Windows, each on x86_64 and Arm.
- **Fixture repositories** built by scripts for each Git situation the app handles: renames, merge conflicts, signed commits, submodules, LFS, a remote that was rewritten, and more. The pictures on this site are rendered from them by the same tests.

## Bugs are reproduced first

A bug counts as fixed only after it is reproduced on the case that was reported, covered by an acceptance scenario whose tests fail before the fix, and the fix is tried on the reported case again. A fix that only makes a symptom disappear does not pass.

## Speed is a rule, not a hope

Rule 16 of the specification reads: *commands cost what Git costs*. A command from the window may take at most 1 % longer than the same Git command typed. Tests compare the Git processes the window starts with those of `git good --cli` for every change it makes, and a nightly job times them. The [performance budgets](performance.md) for large repositories are measured the same way.

## Why say so

You are trusting a Git client with your work, so you should know how it is made. The safeguards above are what we rely on, and they are what make an AI-written app worth trusting, or not. Read the specifications, run the tests, and [open an issue](https://github.com/baol/gg/issues) when something is wrong.
