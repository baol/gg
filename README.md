# gg · good git

Purpose: introduce gg and explain installation and use.

A CLI-first Git browser with a native Rust interface.

```sh
cargo install --path apps/git-good --locked
cd /path/to/repository
git good
git good history
```

`git good` (or `git-good`) opens the containing repository, including from nested directories. Supply a path to open another repository.

Stream, filter, and inspect history; copy commit IDs. `git good history` streams JSON for scripts.

[Contributing](CONTRIBUTING.md) · [Development](docs/development.md) · [Architecture](docs/architecture.md) · [Design](docs/design.md) · [Plan](docs/plan.md) · [Roadmap](ROADMAP.md) · [Regression coverage](docs/regression.md) · [Verification](docs/verification.md)

Inspiration: [Fork](https://fork.dev/) workflows and PhotoCraft's [guide](https://github.com/storytold/photocraft/blob/main/AGENTS.md) and [architecture](https://github.com/storytold/photocraft/blob/main/docs/architecture.md).

Apache-2.0 · [LICENSE](LICENSE)
