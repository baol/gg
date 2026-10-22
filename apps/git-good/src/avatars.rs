//! Lazy, bounded avatar fetching with a fixed-size local fallback.
use eframe::egui;
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc,
    time::Duration,
};

const CACHE_SIZE: usize = 128;
type Reply = (String, Option<egui::ColorImage>);

struct Portrait {
    source: egui::ColorImage,
    textures: HashMap<usize, egui::TextureHandle>,
}

impl Portrait {
    fn new(source: egui::ColorImage) -> Self {
        Self {
            source,
            textures: HashMap::new(),
        }
    }
}

#[derive(Default)]
pub struct Avatars {
    entries: HashMap<String, (Option<Portrait>, u64)>,
    clock: u64,
    allow_updates: bool,
    worker: Option<(mpsc::SyncSender<String>, mpsc::Receiver<Reply>)>,
    failed: bool,
}

pub fn key(email: &str) -> String {
    Sha256::digest(email.trim().to_lowercase().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

const MAX_BYTES: u64 = 256 * 1024;

fn cache_root() -> Option<PathBuf> {
    let base = if cfg!(target_os = "windows") {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Caches"))
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
    };
    base.map(|base| base.join("git-good/avatars-v2-128"))
}

fn decode(bytes: &[u8]) -> Option<egui::ColorImage> {
    if bytes.len() as u64 > MAX_BYTES {
        return None;
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(256);
    limits.max_image_height = Some(256);
    limits.max_alloc = Some(1024 * 1024);
    reader.limits(limits);
    let image = reader.decode().ok()?.into_rgba8();
    Some(egui::ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

// A successful response contains encoded image bytes; None is an authoritative 404.
fn fetch(client: &reqwest::blocking::Client, key: &str) -> Result<Option<Vec<u8>>, ()> {
    let response = client
        .get(format!(
            "https://www.gravatar.com/avatar/{key}?s=128&d=404&r=g"
        ))
        .send()
        .map_err(|_| ())?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response.error_for_status().map_err(|_| ())?;
    let mut bytes = Vec::new();
    response
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    if decode(&bytes).is_none() {
        return Err(());
    }
    Ok(Some(bytes))
}

fn read_cache(path: &Path) -> Option<Option<egui::ColorImage>> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .ok()?
        .take(MAX_BYTES + 2)
        .read_to_end(&mut bytes)
        .ok()?;
    match bytes.split_first()? {
        (0, []) => Some(None),
        (1, image) => decode(image).map(Some),
        _ => None,
    }
}

fn write_cache(path: &Path, bytes: Option<&[u8]>) -> std::io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| std::io::Error::other("Missing cache directory"))?;
    std::fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(&[u8::from(bytes.is_some())])?;
    if let Some(bytes) = bytes {
        file.write_all(bytes)?;
    }
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}

fn cached_image(
    root: Option<&Path>,
    key: &str,
    attempted: &mut HashSet<String>,
    fetch: impl FnOnce() -> Result<Option<Vec<u8>>, ()>,
) -> Option<egui::ColorImage> {
    let path = root.map(|root| root.join(format!("{key}.avatar")));
    if let Some(cached) = path.as_deref().and_then(read_cache) {
        return cached;
    }
    if !attempted.insert(key.to_owned()) {
        return None;
    }
    let bytes = fetch().ok()?;
    let image = match &bytes {
        Some(bytes) => Some(decode(bytes)?),
        None => None,
    };
    if let Some(path) = path {
        let _ = write_cache(&path, bytes.as_deref());
    }
    image
}

impl Avatars {
    pub fn paint(&mut self, ui: &egui::Ui, email: &str, author: &str, rect: egui::Rect) {
        if let Some(texture) = self.texture(ui.ctx(), email, rect.height()) {
            ui.painter().image(
                texture,
                rect,
                egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        } else {
            ui.painter()
                .rect_filled(rect, 3.0, ui.visuals().widgets.inactive.bg_fill);
            let initial = author
                .chars()
                .find(|c| !c.is_whitespace())
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            ui.painter().with_clip_rect(rect).text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                initial,
                egui::FontId::proportional(rect.height() * 0.6),
                ui.visuals().text_color(),
            );
        }
    }

    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        email: &str,
        author: &str,
        size: f32,
    ) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        self.paint(ui, email, author, rect);
        response
    }

    pub fn poll(&mut self, _ctx: &egui::Context, allow_updates: bool) {
        self.allow_updates = allow_updates;
        if !allow_updates {
            return;
        }
        if let Some((_, rx)) = &self.worker {
            while let Ok((key, image)) = rx.try_recv() {
                if let Some(entry) = self.entries.get_mut(&key) {
                    entry.0 = image.map(Portrait::new);
                }
            }
        }
    }

    pub fn texture(
        &mut self,
        ctx: &egui::Context,
        email: &str,
        points: f32,
    ) -> Option<egui::TextureId> {
        let key = key(email);
        self.clock = self.clock.saturating_add(1);
        if let Some((entry, used)) = self.entries.get_mut(&key) {
            *used = self.clock;
            let portrait = entry.as_mut()?;
            let size = crate::thumbnails::physical_size(points, ctx.pixels_per_point());
            if !portrait.textures.contains_key(&size) && portrait.textures.len() >= 2 {
                portrait.textures.clear();
            }
            return Some(
                portrait
                    .textures
                    .entry(size)
                    .or_insert_with(|| {
                        ctx.load_texture(
                            format!("avatar-{key}-{size}"),
                            crate::thumbnails::resize(&portrait.source, size),
                            egui::TextureOptions::LINEAR,
                        )
                    })
                    .id(),
            );
        }
        if cfg!(test) || email.trim().is_empty() || self.failed {
            return None;
        }
        if self.worker.is_none() {
            let (tx, requests) = mpsc::sync_channel::<String>(8);
            let (replies, rx) = mpsc::sync_channel::<Reply>(8);
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("gg-avatars".into())
                .spawn(move || {
                    let Ok(client) = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .redirect(reqwest::redirect::Policy::none())
                        .user_agent("git-good")
                        .build()
                    else {
                        return;
                    };
                    let root = cache_root();
                    let mut attempted = HashSet::new();
                    while let Ok(key) = requests.recv() {
                        let image = cached_image(root.as_deref(), &key, &mut attempted, || {
                            if std::env::var_os("GG_OFFLINE").is_some() {
                                Err(())
                            } else {
                                fetch(&client, &key)
                            }
                        });
                        if replies.send((key, image)).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                });
            if spawned.is_err() {
                self.failed = true;
                return None;
            }
            self.worker = Some((tx, rx));
        }
        if self.entries.len() >= CACHE_SIZE {
            if !self.allow_updates {
                return None;
            }
            if let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, used))| used)
                .map(|(key, _)| key.clone())
            {
                self.entries.remove(&oldest);
            }
        }
        if let Some((tx, _)) = &self.worker {
            match tx.try_send(key.clone()) {
                Ok(()) => {
                    self.entries.insert(key, (None, self.clock));
                }
                Err(mpsc::TrySendError::Disconnected(_)) => self.failed = true,
                Err(mpsc::TrySendError::Full(_)) => {}
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn normalized_email_uses_stable_sha256_key() {
        assert_eq!(key(" User@Example.org "), key("user@example.org"));
        assert_eq!(
            key(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
    #[test]
    fn arriving_avatar_waits_until_interaction_ends() {
        let ctx = egui::Context::default();
        let (requests, _request_rx) = mpsc::sync_channel(1);
        let (replies, rx) = mpsc::sync_channel(1);
        let mut avatars = Avatars {
            worker: Some((requests, rx)),
            ..Default::default()
        };
        avatars.entries.insert("author".into(), (None, 0));
        assert!(
            replies
                .send((
                    "author".into(),
                    Some(egui::ColorImage::filled([2, 2], egui::Color32::WHITE))
                ))
                .is_ok()
        );
        avatars.poll(&ctx, false);
        assert!(
            avatars
                .entries
                .get("author")
                .is_some_and(|entry| entry.0.is_none())
        );
        avatars.poll(&ctx, true);
        assert!(
            avatars
                .entries
                .get("author")
                .is_some_and(|entry| entry.0.is_some())
        );
    }
    fn png() -> Result<Vec<u8>, image::ImageError> {
        let image = image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png)?;
        Ok(bytes.into_inner())
    }

    #[test]
    fn successful_avatar_is_fetched_once_and_survives_restart()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let bytes = png()?;
        let mut calls = 0;
        let author = key(" User@Example.org ");
        for _ in 0..3 {
            // New attempt set models a new process or worker; GPU eviction also takes this path.
            let image = cached_image(Some(dir.path()), &author, &mut HashSet::new(), || {
                calls += 1;
                Ok(Some(bytes.clone()))
            });
            assert_eq!(image.map(|image| image.size), Some([2, 2]));
        }
        assert_eq!(calls, 1);
        Ok(())
    }

    #[test]
    fn missing_avatar_is_negatively_cached_across_restarts()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let mut calls = 0;
        for _ in 0..3 {
            assert!(
                cached_image(
                    Some(dir.path()),
                    &key("missing@example.org"),
                    &mut HashSet::new(),
                    || {
                        calls += 1;
                        Ok(None)
                    }
                )
                .is_none()
            );
        }
        assert_eq!(calls, 1);
        Ok(())
    }

    #[test]
    fn transient_errors_are_attempted_once_per_worker_and_retry_next_session()
    -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let author = key("author@example.org");
        let mut calls = 0;
        let mut attempted = HashSet::new();
        for _ in 0..3 {
            assert!(
                cached_image(Some(dir.path()), &author, &mut attempted, || {
                    calls += 1;
                    Err(())
                })
                .is_none()
            );
        }
        assert_eq!(calls, 1);
        assert!(!dir.path().join(format!("{author}.avatar")).exists());
        let _ = cached_image(Some(dir.path()), &author, &mut HashSet::new(), || {
            calls += 1;
            Ok(None)
        });
        assert_eq!(calls, 2);
        Ok(())
    }

    #[test]
    fn corrupt_cache_is_replaced_by_valid_result() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempfile::tempdir()?;
        let author = key("author@example.org");
        let path = dir.path().join(format!("{author}.avatar"));
        std::fs::write(&path, [1, 2, 3])?;
        let bytes = png()?;
        assert!(
            cached_image(Some(dir.path()), &author, &mut HashSet::new(), || Ok(Some(
                bytes
            )))
            .is_some()
        );
        assert!(read_cache(&path).is_some_and(|entry| entry.is_some()));
        Ok(())
    }

    #[test]
    fn unavailable_disk_still_deduplicates_network_requests() {
        let mut attempted = HashSet::new();
        let mut calls = 0;
        for _ in 0..3 {
            let _ = cached_image(None, &key("author@example.org"), &mut attempted, || {
                calls += 1;
                Ok(None)
            });
        }
        assert_eq!(calls, 1);
    }
    #[test]
    fn list_and_details_use_display_sized_textures_with_reserved_slots() {
        let ctx = egui::Context::default();
        let mut avatars = Avatars::default();
        let email = "author@example.org";
        avatars.entries.insert(
            key(email),
            (
                Some(Portrait::new(egui::ColorImage::filled(
                    [128, 128],
                    egui::Color32::WHITE,
                ))),
                0,
            ),
        );
        let mut output = ctx.run_ui(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                for size in [18.0, 64.0] {
                    let response = avatars.show(ui, email, "Author", size);
                    assert_eq!(response.rect.size(), egui::vec2(size, size));
                }
            });
        });
        assert_eq!(avatars.entries.len(), 1);
        let sizes: Vec<_> = output
            .textures_delta
            .set
            .values()
            .flat_map(|deltas| deltas.iter().map(|delta| delta.image.size()))
            .collect();
        assert!(
            sizes.contains(&[18, 18]) && sizes.contains(&[64, 64]),
            "thumbnail sizes: {sizes:?}"
        );
        output.textures_delta.clear();
        let portrait = avatars.entries[&key(email)]
            .0
            .as_ref()
            .ok_or("missing portrait");
        assert!(portrait.is_ok());
        if let Ok(portrait) = portrait {
            assert_eq!(portrait.textures.len(), 2);
        }
    }
}
