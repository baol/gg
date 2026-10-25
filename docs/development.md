# Development

Purpose: provide build, test, and diagnostic commands.

Use Rust 1.95+ and native graphics drivers. [Installation and usage](../README.md).

```sh
cargo run -p git-good
cargo run -p git-good -- history
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

CI checks and builds six native targets: Linux, macOS, and Windows on x86_64 and arm64. Archives include README, license, and SHA-256 sidecars. Linux targets Ubuntu 24.04 glibc; macOS and Windows archives are unsigned.

Each successful `main` build publishes a regular GitHub release after all six targets pass and their assets verify. `MIN_VERSION` sets the `MAJOR.MINOR.PATCH` floor. Choose the greater of that floor and the highest `vMAJOR.MINOR.PATCH` tag's next patch; use the floor for the first release. Raise the floor to start a minor or major series. Ignore legacy commit tags when numbering.

Serialize release runs with GitHub's [concurrency queue](https://docs.github.com/en/actions/how-tos/write-workflows/choose-when-workflows-run/control-workflow-concurrency) (up to 100 pending runs). Publish the version tag only after successful builds. Reuse an existing commit's version on retry and verify the tag's commit before uploading. Explicit version tags must meet the floor. Pull requests and manual runs produce artifacts without publishing.

Apply the chosen version to the build checkout's workspace manifest and local lockfile entries before compiling. All binaries report it through `git good --version`; CI verifies that output. Source manifests retain the development baseline. Run `python3 -m unittest discover -s scripts -p 'test_*.py'` to check numbering and manifest updates. Supported-platform acceptance remains in the [roadmap](../ROADMAP.md).

The native UI uses egui/eframe 0.37 on wgpu 30. wgpu enables Vulkan, Metal, DX12, and GLES; Linux supports Wayland and X11. The vendored Phosphor font and its license live in `apps/git-good/assets/`.

| Variable | Effect |
| --- | --- |
| `GG_OFFLINE=1` | Use cached avatars and initials |
| `GG_SCREENSHOT=/absolute/path.ppm` | Debug build: capture after loading, then exit |
| `GG_SCREENSHOT_DELAY_MS=500` | Debug build: settle time before the screenshot |
| `GG_WINDOW_SIZE=720x480` | Debug window size |
| `GG_LOCAL_CHANGES=1` | Debug: open Local Changes after history loads |
| `GG_DETAILS=1` | Debug narrow-screen details view |
| `GG_THEME=light` | Debug light-theme layout and contrast inspection |

Use an isolated display for captures. Avatar cache: `git-good/avatars-v2-128` under the OS cache directory (XDG cache, macOS Library/Caches, or Windows LocalAppData).

### Native screenshot hook

Enable the tracked pre-commit hook once per clone:

```sh
git config core.hooksPath .githooks
```

When staged files under `apps/git-good/src`, `apps/git-good/assets`, or the workspace manifests change, the hook builds the debug app, captures `docs/screenshots/gg-native-wide.png` at 1120×740 on an isolated display, and stages the PNG. It needs Python 3; Linux also needs `xvfb-run`. macOS and Windows use the active desktop session.

The hook skips during merge, rebase, cherry-pick, and revert operations and when `GG_SKIP_SCREENSHOT=1` or `CI` is set. It refuses to capture when the staged UI files also have unstaged or untracked edits, so the image matches the commit. Refresh manually with:

```sh
python3 scripts/capture_native_screenshot.py --force
```

Measure a packed 100,000-commit fixture:

```sh
GG_OFFLINE=1 GG_TEST_REPOSITORY=/path/to/fixture cargo test -p git-good packed_history_first_frame_and_deep_navigation -- --ignored --nocapture
```

This opt-in check records first rows, indexing, in-memory frame latency, and deep/back navigation after eviction. Record results in [verification](verification.md).

Measure M1 with a packed 10,000-file change and a 1-MiB text comparison:

```sh
python3 scripts/review-fixture.py /tmp/gg-review-benchmark
GG_OFFLINE=1 GG_REVIEW_REPOSITORY=/tmp/gg-review-benchmark cargo test --release -p git-good packed_review_latency_and_navigation -- --ignored --nocapture
```

Use a new fixture directory. The check exercises production workers and egui frames, asserts latency/cache targets, and visits the last page and back. Record process peak RSS separately; distinguish filesystem-cache conditions from fresh application state.

Stress fully changed text, maximum-pixel PNGs, and rapid selection:

```sh
python3 scripts/review-stress-fixture.py /tmp/gg-review-stress
GG_OFFLINE=1 GG_STRESS_REPOSITORY=/tmp/gg-review-stress cargo test --release -p git-good packed_diff_stress_and_rapid_selection -- --ignored --nocapture
```

On Linux, measure native selection response on an isolated X11 display with Xvfb, libX11, and libXtst installed:

```sh
cargo build --release -p git-good --locked
env -u WAYLAND_DISPLAY xvfb-run -a -s '-screen 0 1280x900x24' python3 scripts/native-review-latency.py target/release/git-good /tmp/gg-review-stress
```

The probe alternates 80 selections while text/image work runs. It polls one pixel for the first visible selection fill, reports p50/p95/max, and checks p95 below 50 ms. Desktop hardware and monitor scanout remain platform checks.
