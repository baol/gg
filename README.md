# gg · good git

Purpose: introduce gg and show how to install and use it.

A CLI-first Git tool with a native Rust interface.

From inside your repository, run `git good` (or `git-good`). gg immediately shows that repository. Nested directories resolve to their containing repository. An optional path selects another repository.

```sh
cargo install --path apps/git-good --locked
cd /path/to/repository
git good
git good history
```

The first feature: browse, filter, and inspect recent commits; copy their IDs. `git good history` prints JSON for scripts.

[Plan](docs/plan.md) · [Roadmap](ROADMAP.md) · [Architecture](docs/architecture.md) · [Development](docs/development.md) · [Contributing](CONTRIBUTING.md) · [Verification](docs/verification.md)

Workflow inspiration: [Fork](https://fork.dev/). Architecture principles: PhotoCraft's [guide](https://github.com/storytold/photocraft/blob/main/AGENTS.md) and [architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).

Apache-2.0 · [LICENSE](LICENSE)
