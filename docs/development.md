# Development

Purpose: provide commands and tooling for building, testing, and debugging gg.

Use Rust 1.94+ and native graphics drivers. Installation and everyday usage: [README](../README.md).

```sh
cargo run -p git-good
cargo run -p git-good -- history
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
```

wgpu enables Vulkan, Metal, DX12, and GLES; Linux windowing supports Wayland and X11. CI tests and builds on Linux, macOS, and Windows.

For visual checks, debug builds accept `GG_SCREENSHOT=/absolute/path.ppm`: load the repository, capture a rendered frame, and exit. Use an isolated display. `GG_WINDOW_SIZE=720x480` sets a debug capture size; `GG_DETAILS=1` opens the narrow-screen details view.

Set `GG_OFFLINE=1` to use cached avatars and local initials without network requests. Avatar files live under `git-good/avatars-v2-128` in the OS cache directory (XDG cache on Linux, Library/Caches on macOS, LocalAppData on Windows).

Run the opt-in end-to-end timing check with a packed 100,000-commit fixture:

```sh
GG_OFFLINE=1 GG_TEST_REPOSITORY=/path/to/fixture cargo test -p git-good packed_history_first_frame_and_deep_navigation -- --ignored --nocapture
```

The check records first rows, completed indexing, and in-memory frame latency; it also visits the oldest row and returns after cache eviction.
