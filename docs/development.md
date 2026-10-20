# Development

Purpose: provide build, test, and diagnostic commands.

Use Rust 1.94+ and native graphics drivers. [Installation and usage](../README.md).

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

wgpu enables Vulkan, Metal, DX12, and GLES; Linux supports Wayland and X11.

| Variable | Effect |
| --- | --- |
| `GG_OFFLINE=1` | Use cached avatars and initials |
| `GG_SCREENSHOT=/absolute/path.ppm` | Debug build: capture after loading, then exit |
| `GG_WINDOW_SIZE=720x480` | Debug window size |
| `GG_DETAILS=1` | Debug narrow-screen details view |

Use an isolated display for captures. Avatar cache: `git-good/avatars-v2-128` under the OS cache directory (XDG cache, macOS Library/Caches, or Windows LocalAppData).

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
