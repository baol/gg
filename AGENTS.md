# Agent instructions

Purpose: direct coding agents to repository rules.

Follow [CONTRIBUTING.md](CONTRIBUTING.md). Read the owning specification before editing and update it when behavior changes.

Name things with the terms in [docs/domain.md](docs/domain.md), in the app, specifications, documentation and conversation. "History" is never used alone: write **History tree**, **File history**, **Line history** or **Rewrite history** in full.

A bug is fixed only after it is reproduced on the reported case, covered by an acceptance scenario in [docs/acceptance.md](docs/acceptance.md) whose tests fail before the fix, and the fix is validated on the reported case again.

## README screenshot

Enable the pre-commit hook once per clone, before the first commit:

```sh
git config core.hooksPath .githooks
```

The hook recaptures `docs/screenshots/gg-native-wide.png` whenever staged UI sources change (`apps/git-good/src`, `apps/git-good/assets`, the manifests, `Cargo.lock`) and stages it with the commit. It renders offscreen (no display server) and needs Python 3 and a wgpu adapter (the GPU, or Mesa's software Vulkan on Linux). Stage every UI edit first: the hook refuses when staged UI files have unstaged or untracked edits.

CI fails when the screenshot was not captured from the commit's UI sources (`python3 scripts/capture_native_screenshot.py --check`). If the hook was skipped, stage the UI changes, run `python3 scripts/capture_native_screenshot.py --force`, and commit the PNG. Do not commit other screenshots. See [development](docs/development.md#native-screenshot-hook).
