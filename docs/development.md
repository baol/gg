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

CI runs checks and release builds on Linux, macOS, and Windows with `GG_OFFLINE=1`. wgpu enables Vulkan, Metal, DX12, and GLES; Linux supports Wayland and X11.

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
