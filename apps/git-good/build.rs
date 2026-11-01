//! Build facts for About gg: the commit, the build date, and the target.
use std::process::Command;

fn main() {
    let commit = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|commit| commit.trim().to_string())
        .filter(|commit| !commit.is_empty())
        .unwrap_or_else(|| "unknown".into());
    // Reproducible builds pin the date; otherwise it is today (UTC).
    let seconds = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs() as i64)
                .unwrap_or_default()
        });
    let target = std::env::var("TARGET").unwrap_or_else(|_| "unknown".into());
    println!("cargo:rustc-env=GG_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=GG_BUILD_DATE={}", date(seconds));
    println!("cargo:rustc-env=GG_BUILD_TARGET={target}");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");
    // Rebuild the facts when HEAD moves.
    for path in ["HEAD", "refs/heads"] {
        if let Some(path) = Command::new("git")
            .args(["rev-parse", "--git-path", path])
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
        {
            println!("cargo:rerun-if-changed={}", path.trim());
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}

/// `YYYY-MM-DD` (UTC) from Unix seconds.
fn date(seconds: i64) -> String {
    let z = seconds.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}
