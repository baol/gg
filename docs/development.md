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

For visual checks, debug builds accept `GG_SCREENSHOT=/absolute/path.ppm`: load the repository, capture a rendered frame, and exit. Use an isolated display.
