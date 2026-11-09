//! The README screenshot, rendered offscreen: the window as it opens on
//! this repository, drawn by egui_kittest through wgpu (a software Vulkan
//! or the platform's own GPU) with no display server and no window on the
//! desktop. `scripts/capture_native_screenshot.py` runs this ignored test
//! and stamps the image with the UI sources it came from.

use crate::Browser;
use eframe::egui;
use std::time::{Duration, Instant};

/// The window's size, as a fresh launch opens it.
const SIZE: egui::Vec2 = egui::vec2(1120.0, 740.0);

/// Write the window's first settled frame on `GG_SCREENSHOT_REPOSITORY`
/// (default: this repository) to `GG_SCREENSHOT` as a binary PPM.
#[test]
#[ignore = "renders the README screenshot: scripts/capture_native_screenshot.py runs it"]
fn readme_screenshot() -> Result<(), Box<dyn std::error::Error>> {
    let out = std::env::var_os("GG_SCREENSHOT").ok_or("set GG_SCREENSHOT to the output path")?;
    let repository = std::env::var_os("GG_SCREENSHOT_REPOSITORY").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."),
        std::path::PathBuf::from,
    );
    let repository = repository.canonicalize()?;
    let mut app = Browser::new(repository);
    // As a launch on Windows and Linux with the default preferences: the
    // menu bar is the title bar, with the caption buttons at the right.
    app.chrome.custom = crate::custom_titlebar(
        &crate::settings::Prefs::default(),
        crate::Desktop::OtherLinux,
    );
    let mut harness = egui_kittest::Harness::builder()
        .with_size(SIZE)
        .with_pixels_per_point(1.0)
        .with_theme(egui::Theme::Dark)
        .with_context_setup(|ctx| {
            crate::phosphor::install(ctx);
            crate::wide_font::install(ctx);
        })
        .allow_missing_glyphs()
        .wgpu()
        .build_ui_state(|ui, app: &mut Browser| app.draw_ui(ui), app);
    let ctx = harness.ctx.clone();
    {
        let app = harness.state_mut();
        app.apply_prefs(&ctx);
        app.details_visible = app.prefs.details_visible;
        app.load(&ctx);
    }
    // Until History has read its first page and the background reads
    // (local summary, signing) are back, then a few frames to settle.
    let started = Instant::now();
    loop {
        harness.step();
        let app = harness.state();
        let settled = app.state.count > 0 && app.local_summary.is_some() && app.pending.is_none();
        if settled && started.elapsed() > Duration::from_millis(500) {
            break;
        }
        if started.elapsed() > Duration::from_secs(60) {
            return Err("the window did not settle in 60 s".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    harness.run_steps(4);
    let image = harness.render()?;
    let mut ppm = format!("P6\n{} {}\n255\n", image.width(), image.height()).into_bytes();
    for pixel in image.pixels() {
        ppm.extend_from_slice(&pixel.0[..3]);
    }
    std::fs::write(out, ppm)?;
    Ok(())
}
