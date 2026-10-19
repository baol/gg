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

CI runs offline checks, release builds, and release CLI smoke tests natively on Linux, macOS, and Windows for x86_64 and arm64. Each job uploads a binary archive with README, license, and a SHA-256 sidecar. Linux binaries target glibc on Ubuntu 24.04; macOS and Windows archives are unsigned. Native graphics and platform runtime acceptance remain gates for supported milestone releases; automatic development prereleases provide binaries for each passing main commit.

Run CI on pull requests, pushes to `main`, version tags (`v*`), and manual dispatch. Cache Cargo dependencies and build outputs per target; cancel superseded pull-request runs and keep platform failures independent. Keep each main commit run so later merges cannot cancel its release.

Every push to `main`, including each merged commit, automatically publishes a GitHub prerelease named `commit-<full SHA>` after all six targets pass. Attach all six binary archives and checksum sidecars after verifying the complete set. Keep releases tied to the exact built commit. Rerunning the job updates the same release without duplicates. Pull requests and manual dispatch run checks and produce artifacts without publishing.

Version-tag pushes (`v*`) automatically publish the tagged release after the same checks. Complete the [milestone release gates](../ROADMAP.md) before tagging a supported release. CI artifacts are also downloadable from successful platform jobs.

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
