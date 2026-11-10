# Documentation site

Purpose: explain how the user documentation site is built and previewed.

The site at <https://baol.github.io/gg/> is built with [Zensical](https://zensical.org) from the pages in `docs/`, configured in `zensical.toml`. The `Docs` workflow (`.github/workflows/docs.yml`) builds it on pull requests that touch it and publishes it to GitHub Pages on every push to `main`.

## Screenshots

The screenshots in `docs/assets/screens/` are rendered from the app and are not committed (see [AGENTS.md](../AGENTS.md)). `scripts/docs_screenshots.py` builds fixture repositories for an example project, `acme-api`, then runs the ignored test `docs_screens::docs_screenshots` (`apps/git-good/src/docs_screens.rs`), which opens the window offscreen in each state and saves a PNG. It needs a wgpu adapter: the GPU, or Mesa's software Vulkan on Linux (`mesa-vulkan-drivers`).

```sh
python3 scripts/docs_screenshots.py                 # all shots
python3 scripts/docs_screenshots.py --only rebase   # some of them
```

To add a shot, set the state in `docs_screens.rs` under a new name, extend the fixture in the script if it needs more history, and reference `assets/screens/<name>.png` from a page. An animation is a `Recording`: call `shot` after each step, `finish` at the end, and reference `assets/screens/<name>.gif`.

## Preview

```sh
pip install zensical pillow
python3 scripts/docs_screenshots.py
cd website && zensical serve
```
