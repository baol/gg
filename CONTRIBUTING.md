# Contributing

Purpose: define how to develop and deliver changes.

- Finish one milestone at a time through small working increments in the [plan](docs/plan.md). Refine behavior and appearance together.
- Follow the [architecture](docs/architecture.md) and [design guidelines](docs/design.md). Add abstractions for concrete responsibilities.
- Write original Rust from public specifications and observed behavior; attribute reused material.
- Write terse, positive requirements in one owning document; link from others. Start each Markdown document with its purpose.
- Preserve the [regression baseline](docs/regression.md). Use Git fixture oracles, production worker/input paths, and in-memory geometry assertions. Start bug fixes with a failing test.
- Run the [development checks](docs/development.md), inspect UI changes natively, and record [evidence](docs/verification.md).
- Use descriptive branch names without prefixes. Keep branch names, commit messages, and PR text free of agent attribution.
- Keep a straight, linear history at all times: no merge commits. Bring in other work with `git rebase` or `git cherry-pick`, update a branch with `git pull --rebase`, and land pull requests with rebase or squash merges. Rewriting a pull-request branch to keep it linear is always allowed with `git push --force-with-lease`; never force-push `main`.
