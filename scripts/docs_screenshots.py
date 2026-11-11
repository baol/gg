#!/usr/bin/env python3
"""Render the documentation site's screenshots into website/docs/assets/screens.

Builds fixture repositories for a small web service ("acme-api") in a
temporary folder (history with several authors, feature branches, tags, an
`origin` remote the local branch is ahead of and behind, working changes, a
stash, and a clone stopped in a merge conflict), points Git Good's settings
at a temporary configuration folder, and runs the ignored test
`docs_screens::docs_screenshots`, which renders each view offscreen.

The PNGs are not committed (see AGENTS.md); the docs workflow runs this
before building the site. `--only history,review` renders some of them.
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "website" / "docs" / "assets" / "screens"

PEOPLE = {
    "ada": ("Ada Lovelace", "ada@acme.test"),
    "grace": ("Grace Hopper", "grace@acme.test"),
    "linus": ("Linus Ortega", "linus@acme.test"),
    "margaret": ("Margaret Hamilton", "margaret@acme.test"),
}

CLOCK = [1_793_000_000]
BINARY: list[Path] = []


def git(repo: Path, *args: str, who: str = "ada", check: bool = True) -> str:
    name, email = PEOPLE[who]
    CLOCK[0] += 5_400
    stamp = f"{CLOCK[0]} +0100"
    env = {
        **os.environ,
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": name,
        "GIT_AUTHOR_EMAIL": email,
        "GIT_COMMITTER_NAME": name,
        "GIT_COMMITTER_EMAIL": email,
        "GIT_AUTHOR_DATE": stamp,
        "GIT_COMMITTER_DATE": stamp,
    }
    result = subprocess.run(
        ["git", "-C", str(repo), *args],
        env=env,
        check=check,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def good(repo: Path, *args: str, who: str = "ada") -> str:
    """Run Git Good's own command line (`git good --cli ...`) in `repo`."""
    name, email = PEOPLE[who]
    env = {
        **os.environ,
        "GIT_CONFIG_NOSYSTEM": "1",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_AUTHOR_NAME": name,
        "GIT_AUTHOR_EMAIL": email,
        "GIT_COMMITTER_NAME": name,
        "GIT_COMMITTER_EMAIL": email,
        "GG_OFFLINE": "1",
    }
    result = subprocess.run(
        [str(BINARY[0]), "--cli", *args, "--repo", str(repo)],
        env=env,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def write(repo: Path, path: str, text: str) -> None:
    target = repo / path
    target.parent.mkdir(parents=True, exist_ok=True)
    target.write_text(text)


def commit(repo: Path, message: str, who: str, files: dict[str, str]) -> str:
    for path, text in files.items():
        write(repo, path, text)
    git(repo, "add", "-A", who=who)
    git(repo, "commit", "-q", "-m", message, who=who)
    return git(repo, "rev-parse", "HEAD")


CARGO = """[package]
name = "acme-api"
version = "0.2.0"
edition = "2024"

[dependencies]
axum = "0.8"
serde = { version = "1.0.210", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
"""

MAIN_V1 = """use axum::{routing::get, Router};

mod routes;

#[tokio::main]
async fn main() {
    let app = Router::new().route("/health", get(routes::health));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
"""

MAIN_V2 = """use axum::{routing::get, Router};

mod routes;

#[tokio::main]
async fn main() {
    let port = std::env::var("ACME_PORT").unwrap_or_else(|_| "8080".into());
    let app = Router::new()
        .route("/health", get(routes::health))
        .route("/orders", get(routes::list_orders));
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
"""

ROUTES_V1 = """use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Health {
    pub status: &'static str,
}

pub async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

pub async fn list_orders() -> Json<Vec<String>> {
    Json(vec!["order-1".into(), "order-2".into()])
}
"""

ROUTES_V0 = """use axum::Json;

pub async fn list_orders() -> Json<Vec<String>> {
    Json(vec!["order-1".into(), "order-2".into()])
}
"""

ROUTES_V2 = ROUTES_V1 + """
pub async fn not_found() -> (axum::http::StatusCode, &'static str) {
    (axum::http::StatusCode::NOT_FOUND, "not found")
}
"""

ROUTES_V3 = ROUTES_V1 + """
#[derive(Serialize)]
pub struct Problem {
    pub status: u16,
    pub title: &'static str,
}

pub async fn not_found() -> (axum::http::StatusCode, Json<Problem>) {
    let problem = Problem { status: 404, title: "Not Found" };
    (axum::http::StatusCode::NOT_FOUND, Json(problem))
}
"""

README = """# acme-api

The order service behind the Acme storefront.

## Configuration

| Variable | Default | Meaning |
| --- | --- | --- |
| `ACME_PORT` | `8080` | Port to listen on |
"""


def build(root: Path) -> dict[str, str]:
    commits: dict[str, str] = {}
    origin = root / "origin.git"
    api = root / "acme-api"
    subprocess.run(["git", "init", "-q", "--bare", "-b", "main", str(origin)], check=True)
    api.mkdir()
    git(api, "init", "-q", "-b", "main")
    git(api, "config", "core.autocrlf", "false")
    commit(api, "Start the acme-api service", "ada", {
        "Cargo.toml": CARGO, "src/main.rs": MAIN_V1, "src/routes.rs": ROUTES_V0,
        "README.md": "# acme-api\n\nThe order service behind the Acme storefront.\n",
    })
    commit(api, "Add the /health endpoint", "grace", {"src/routes.rs": ROUTES_V1})
    commits["port"] = commit(api, "Read the port from ACME_PORT", "linus", {"src/main.rs": MAIN_V2})
    git(api, "tag", "-a", "v0.1.0", "-m", "First release", who="ada")
    commit(api, "Add request logging middleware", "margaret", {
        "src/logging.rs": "pub fn init() {\n    tracing_subscriber::fmt::init();\n}\n",
    })
    commit(api, "Answer unknown paths with 404", "grace", {"src/routes.rs": ROUTES_V2})
    commits["not_found_json"] = commit(api, "Return 404 as JSON", "ada", {"src/routes.rs": ROUTES_V3})
    git(api, "tag", "-a", "v0.2.0", "-m", "Problem responses", who="ada")
    commit(api, "Document the configuration", "margaret", {"README.md": README})
    for who, message, path, text in [
        ("linus", "Add a Dockerfile", "Dockerfile", "FROM rust:1.82 AS build\nWORKDIR /app\nCOPY . .\nRUN cargo build --release\n"),
        ("grace", "Run tests in CI", ".github/workflows/ci.yml", "name: CI\non: [push]\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - run: cargo test\n"),
        ("margaret", "Add a CHANGELOG", "CHANGELOG.md", "# Changelog\n\n## 0.2.0\n\n- Problem responses for 404s\n"),
        ("ada", "Explain how to run locally", "docs/running.md", "# Running locally\n\n    ACME_PORT=9000 cargo run\n"),
        ("linus", "Ignore build output", ".gitignore", "/target\n"),
        ("grace", "Pin the toolchain", "rust-toolchain.toml", "[toolchain]\nchannel = \"1.82\"\n"),
    ]:
        commit(api, message, who, {path: text})
    git(api, "remote", "add", "origin", str(origin))
    git(api, "push", "-q", "-u", "origin", "main")

    # Feature branches, one with a stash on it.
    git(api, "switch", "-q", "-c", "feature/rate-limit")
    bucket = commit(api, "Add a token-bucket rate limiter", "linus", {
        "src/limit.rs": "pub struct Bucket {\n    pub capacity: u32,\n    pub tokens: u32,\n}\n",
    })
    limit = commit(api, "Limit /orders to 20 requests a second", "linus", {
        "src/main.rs": MAIN_V2.replace('get(routes::list_orders))', 'get(routes::list_orders).layer(limit::per_second(20)))'),
    })
    # A reword through Git Good's own rewrite, which leaves a backup for
    # the Recovery window to show.
    good(api, "rewrite", "--from", bucket, "--pick", bucket,
         "--reword", limit, "Limit /orders to 20 requests per second", who="linus")
    write(api, "src/limit.rs", "pub struct Bucket {\n    pub capacity: u32,\n    pub tokens: u32,\n    pub burst: u32,\n}\n")
    git(api, "stash", "push", "-q", "-m", "tune the burst size", who="linus")
    git(api, "push", "-q", "-u", "origin", "feature/rate-limit")
    git(api, "switch", "-q", "main")
    git(api, "switch", "-q", "-c", "fix/timeout")
    commit(api, "Time out upstream calls after 5 s", "grace", {
        "src/upstream.rs": "pub const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);\n",
    })
    git(api, "switch", "-q", "main")
    git(api, "merge", "-q", "--no-ff", "fix/timeout", "-m", "Merge branch 'fix/timeout'", who="ada")

    # Someone else pushed meanwhile: main is ahead and behind origin.
    other = root / "teammate"
    subprocess.run(["git", "clone", "-q", str(origin), str(other)], check=True, capture_output=True)
    commit(other, "Bump serde to 1.0.214", "margaret", {"Cargo.toml": CARGO.replace("1.0.210", "1.0.214")})
    git(other, "push", "-q", "origin", "main", who="margaret")
    git(api, "fetch", "-q", "origin")
    commits["rebase_from"] = commit(api, "Read the database URL from ACME_DB", "linus", {
        "src/db.rs": "pub fn url() -> String {\n    std::env::var(\"ACME_DB\").unwrap_or_else(|_| \"postgres://localhost/acme\".into())\n}\n",
    })
    commit(api, "WIP fix db typo", "linus", {
        "src/db.rs": "pub fn url() -> String {\n    std::env::var(\"ACME_DB\").unwrap_or_else(|_| \"postgres://localhost/acme\".into())\n}\n\npub const POOL: u32 = 8;\n",
    })
    commit(api, "Log the listening address", "ada", {
        "src/main.rs": MAIN_V2.replace("    axum::serve", "    println!(\"listening on {port}\");\n    axum::serve"),
    })

    # Work in progress: a staged edit, two unstaged hunks, a new file.
    write(api, "README.md", README + "| `ACME_TIMEOUT_MS` | `5000` | Upstream timeout |\n")
    git(api, "add", "README.md")
    write(api, "src/routes.rs", ROUTES_V3.replace(
        'Json(vec!["order-1".into(), "order-2".into()])',
        'Json(orders::recent(50).await)',
    ).replace("use serde::Serialize;", "use serde::Serialize;\n\nuse crate::orders;"))
    write(api, "src/orders.rs", "pub async fn recent(limit: usize) -> Vec<String> {\n    (1..=limit).map(|n| format!(\"order-{n}\")).collect()\n}\n")

    # A linked worktree for the hotfix, beside the main one.
    git(api, "worktree", "add", "-q", "-b", "hotfix/1.2.1", str(root / "acme-api-hotfix"), "v0.2.0")

    # A clone stopped in a merge conflict on src/routes.rs.
    merge = root / "acme-api-merge"
    subprocess.run(["git", "clone", "-q", str(origin), str(merge)], check=True, capture_output=True)
    git(merge, "config", "core.autocrlf", "false")
    commit(merge, "Page the order list", "grace", {"src/routes.rs": ROUTES_V3.replace(
        'Json(vec!["order-1".into(), "order-2".into()])', 'Json(page(1, 25))')})
    git(merge, "switch", "-q", "-c", "feature/sorting", "HEAD~1")
    commit(merge, "Sort orders newest first", "linus", {"src/routes.rs": ROUTES_V3.replace(
        'Json(vec!["order-1".into(), "order-2".into()])', 'Json(newest_first(vec!["order-2".into(), "order-1".into()]))')})
    git(merge, "switch", "-q", "main")
    git(merge, "merge", "feature/sorting", who="grace", check=False)
    return commits


def big(root: Path, count: int = 20_000) -> Path:
    """A repository with `count` commits by four people, release branches
    and tags, written with `git fast-import` in a second or two."""
    path = root / "monorepo"
    git(path.parent, "init", "-q", "-b", "main", str(path))
    people = [("Ada Lovelace", "ada"), ("Grace Hopper", "grace"),
              ("Linus Ortega", "linus"), ("Margaret Hamilton", "margaret")]
    verbs = ["Fix", "Speed up", "Document", "Refactor", "Test", "Tidy", "Rename", "Guard"]
    areas = ["parser", "scheduler", "cache", "router", "billing", "search", "auth", "metrics"]
    start = 1_700_000_000
    chunks = []
    for i in range(1, count + 1):
        name, login = people[i % len(people)]
        when = start + i * 1_800
        message = f"{verbs[i % len(verbs)]} the {areas[(i // 7) % len(areas)]} ({i})"
        body = f"pub const STEP: u32 = {i};\n"
        chunks.append(
            f"commit refs/heads/main\nmark :{i}\n"
            f"author {name} <{login}@example.com> {when} +0000\n"
            f"committer {name} <{login}@example.com> {when} +0000\n"
            f"data {len(message.encode())}\n{message}\n"
            + (f"from :{i - 1}\n" if i > 1 else "")
            + f"M 644 inline src/{areas[i % len(areas)]}/mod{i % 40}.rs\n"
            f"data {len(body.encode())}\n{body}\n"
        )
        if i % 2_500 == 0:
            chunks.append(f"reset refs/heads/release/{i // 2_500}\nfrom :{i}\n\n")
            chunks.append(f"reset refs/tags/v{i // 2_500}.0.0\nfrom :{i}\n\n")
    subprocess.run(["git", "-C", str(path), "fast-import", "--quiet"],
                   input="".join(chunks).encode(), check=True)
    git(path, "reset", "-q", "--hard", "main")
    return path


def encode_animations(out: Path) -> None:
    """Turn each NAME.frames folder the shots wrote (PNG frames and
    frames.json with each frame's duration in ms) into NAME.gif, 1280 px
    wide with one shared palette, and remove the frames."""
    from PIL import Image

    for frames in sorted(out.glob("*.frames")):
        durations = json.loads((frames / "frames.json").read_text())
        images = []
        for index in range(len(durations)):
            image = Image.open(frames / f"{index:03}.png").convert("RGB")
            width = 1280
            images.append(image.resize((width, image.height * width // image.width), Image.LANCZOS))
        palette = images[0].quantize(colors=255, method=Image.Quantize.MEDIANCUT)
        quantized = [image.quantize(palette=palette, dither=Image.Dither.NONE) for image in images]
        target = out / (frames.name.removesuffix(".frames") + ".gif")
        quantized[0].save(target, save_all=True, append_images=quantized[1:],
                          duration=durations, loop=0, optimize=True, disposal=1)
        for file in frames.iterdir():
            file.unlink()
        frames.rmdir()
        print(f"wrote {target} ({len(durations)} frames, {target.stat().st_size // 1024} KiB)")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--only", help="comma-separated shot names")
    parser.add_argument("--out", type=Path, default=OUT)
    arguments = parser.parse_args(argv)
    # The fixture uses the app's own command line for its rewrites.
    subprocess.run(["cargo", "build", "-q", "-p", "git-good", "--locked"], cwd=ROOT, check=True)
    target = Path(os.environ.get("CARGO_TARGET_DIR", ROOT / "target"))
    BINARY.append(target / "debug" / ("git-good.exe" if os.name == "nt" else "git-good"))
    with tempfile.TemporaryDirectory(prefix="gg-docs-") as directory:
        root = Path(directory)
        commits = build(root)
        big(root)
        # The demos that commit and rewrite start from a copy with the work
        # in progress put away, so the other shots keep their state.
        shutil.copytree(root / "acme-api", root / "acme-api-demo", symlinks=True)
        git(root / "acme-api-demo", "stash", "push", "-q", "-u", "-m", "demo")
        # Git Good's own Git commits there, as a person would, with their identity.
        git(root / "acme-api-demo", "config", "user.name", "Ada Lovelace")
        git(root / "acme-api-demo", "config", "user.email", PEOPLE["ada"][1])
        config = root / ".config"
        (config / "gg").mkdir(parents=True)
        recent = [
            {"path": str(root / "acme-api"), "opened": 1_793_100_000},
            {"path": str(root / "acme-api-merge"), "opened": 1_793_000_000},
            {"path": str(root / "teammate"), "opened": 1_792_500_000},
        ]
        (root / "src").mkdir()
        (config / "gg" / "settings.json").write_text(json.dumps({
            "recent_repositories": recent,
            "clone_folders": {"roots": [str(root / "src")], "preferred": str(root / "src")},
        }))
        manifest = root / "fixtures.json"
        manifest.write_text(json.dumps({"root": str(root), "commits": commits}))
        environment = {
            **os.environ,
            # The window title and welcome list show ~/acme-api, not the
            # temporary folder; cargo and rustup keep their own homes.
            "HOME": str(root),
            "CARGO_HOME": os.environ.get("CARGO_HOME", str(Path.home() / ".cargo")),
            "RUSTUP_HOME": os.environ.get("RUSTUP_HOME", str(Path.home() / ".rustup")),
            "XDG_CONFIG_HOME": str(config),
            "APPDATA": str(config),
            "GIT_CONFIG_NOSYSTEM": "1",
            "GIT_CONFIG_GLOBAL": os.devnull,
            "GG_OFFLINE": "1",
            "GG_DOCS_FIXTURES": str(manifest),
            "GG_DOCS_SCREENS": str(arguments.out.resolve()),
        }
        if arguments.only:
            environment["GG_DOCS_ONLY"] = arguments.only
        command = [
            "cargo", "test", "-q", "-p", "git-good", "--bin", "git-good", "--locked",
            "--", "--ignored", "--exact", "--nocapture", "docs_screens::docs_screenshots",
        ]
        status = subprocess.run(command, cwd=ROOT, env=environment).returncode
        if status == 0:
            encode_animations(arguments.out.resolve())
        return status


if __name__ == "__main__":
    sys.exit(main())
