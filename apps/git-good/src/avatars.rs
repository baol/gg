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

/// Whether avatars may be fetched from the network; off uses the cache and
/// initials. Set from Preferences (and `GG_OFFLINE`).
static NETWORK: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);

pub fn set_network(enabled: bool) {
    NETWORK.store(enabled, std::sync::atomic::Ordering::Relaxed);
}

/// The avatar sources that are on, as a bit per [`AvatarSource::ALL`] entry.
static SOURCES: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0b0101);

/// Turn avatar sources on or off, from Preferences.
pub fn set_sources(sources: &[AvatarSource]) {
    SOURCES.store(mask(sources), std::sync::atomic::Ordering::Relaxed);
}

fn mask(sources: &[AvatarSource]) -> u8 {
    AvatarSource::ALL
        .iter()
        .enumerate()
        .filter(|(_, source)| sources.contains(source))
        .fold(0, |mask, (bit, _)| mask | (1 << bit))
}

use crate::settings::AvatarSource;

/// One way to find an avatar.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Lookup {
    /// An image URL; a 404 means "no avatar here".
    Image(String),
    /// GitLab's `/api/v4/avatar` answer names the image URL.
    GitlabJson(String),
}

/// The lookups for `email`, in source order, for the sources in `mask`.
fn lookups(email: &str, key: &str, mask: u8) -> Vec<Lookup> {
    let email = email.trim();
    let on = |source: AvatarSource| {
        AvatarSource::ALL
            .iter()
            .position(|candidate| *candidate == source)
            .is_some_and(|bit| mask & (1 << bit) != 0)
    };
    let mut out = Vec::new();
    if on(AvatarSource::Github)
        && let Some(local) = email
            .to_lowercase()
            .strip_suffix("@users.noreply.github.com")
            .map(str::to_owned)
    {
        // `12345+name@…` names the account ID; older addresses name the user.
        let id = local.split_once('+').map(|(id, _)| id.to_owned());
        match id.filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())) {
            Some(id) => out.push(Lookup::Image(format!(
                "https://avatars.githubusercontent.com/u/{id}?s=128"
            ))),
            None if !local.is_empty()
                && local
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-') =>
            {
                out.push(Lookup::Image(format!(
                    "https://avatars.githubusercontent.com/{local}?s=128"
                )));
            }
            None => {}
        }
    }
    if on(AvatarSource::Gitlab) && email.contains('@') {
        out.push(Lookup::GitlabJson(format!(
            "https://gitlab.com/api/v4/avatar?email={}&size=128",
            percent_encode(email)
        )));
    }
    if on(AvatarSource::Gravatar) {
        out.push(Lookup::Image(format!(
            "https://www.gravatar.com/avatar/{key}?s=128&d=404&r=g"
        )));
    }
    if on(AvatarSource::Libravatar) {
        out.push(Lookup::Image(format!(
            "https://seccdn.libravatar.org/avatar/{key}?s=128&d=404"
        )));
    }
    out
}

fn percent_encode(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || b"-._~".contains(&byte) {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}
type Reply = (String, Option<egui::ColorImage>);
/// An avatar request: the email's cache key and the email itself.
type Request = (String, String);

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
    worker: Option<(mpsc::SyncSender<Request>, mpsc::Receiver<Reply>)>,
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
fn fetch_image(client: &reqwest::blocking::Client, url: &str) -> Result<Option<Vec<u8>>, ()> {
    let response = client.get(url).send().map_err(|_| ())?;
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

/// Try each lookup in order; the first image wins. `Ok(None)` only when
/// every source answered that it has none, so a network error is retried
/// on a later launch instead of being cached as "no avatar".
fn fetch(client: &reqwest::blocking::Client, lookups: &[Lookup]) -> Result<Option<Vec<u8>>, ()> {
    let mut failed = false;
    for lookup in lookups {
        let result = match lookup {
            Lookup::Image(url) => fetch_image(client, url),
            Lookup::GitlabJson(url) => gitlab_image(client, url),
        };
        match result {
            Ok(Some(bytes)) => return Ok(Some(bytes)),
            Ok(None) => {}
            Err(()) => failed = true,
        }
    }
    if failed { Err(()) } else { Ok(None) }
}

/// GitLab answers `{"avatar_url": …}`; its fallback to Gravatar is left to
/// the Gravatar source, so only a GitLab-hosted image counts.
fn gitlab_image(client: &reqwest::blocking::Client, url: &str) -> Result<Option<Vec<u8>>, ()> {
    let response = client.get(url).send().map_err(|_| ())?;
    let response = response.error_for_status().map_err(|_| ())?;
    let mut bytes = Vec::new();
    response
        .take(16 * 1024)
        .read_to_end(&mut bytes)
        .map_err(|_| ())?;
    let answer: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ())?;
    match answer.get("avatar_url").and_then(serde_json::Value::as_str) {
        Some(image) if image.starts_with("https://") && !image.contains("gravatar.com") => {
            fetch_image(client, image)
        }
        _ => Ok(None),
    }
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
            let (tx, requests) = mpsc::sync_channel::<Request>(8);
            let (replies, rx) = mpsc::sync_channel::<Reply>(8);
            let ctx = ctx.clone();
            let spawned = std::thread::Builder::new()
                .name("gg-avatars".into())
                .spawn(move || {
                    let Ok(client) = reqwest::blocking::Client::builder()
                        .timeout(Duration::from_secs(3))
                        .redirect(reqwest::redirect::Policy::limited(3))
                        .user_agent("git-good")
                        .build()
                    else {
                        return;
                    };
                    let root = cache_root();
                    let mut attempted = HashSet::new();
                    while let Ok((key, email)) = requests.recv() {
                        // The cache entry names the sources it was found with,
                        // so turning a source on looks again.
                        let mask = SOURCES.load(std::sync::atomic::Ordering::Relaxed);
                        let entry = format!("{key}-{mask}");
                        let image = cached_image(root.as_deref(), &entry, &mut attempted, || {
                            if !NETWORK.load(std::sync::atomic::Ordering::Relaxed) {
                                Err(())
                            } else {
                                fetch(&client, &lookups(&email, &key, mask))
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
            match tx.try_send((key.clone(), email.to_string())) {
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
    fn lookups_follow_the_enabled_sources_in_order() {
        let all = mask(&AvatarSource::ALL);
        let urls = |email: &str, mask: u8| lookups(email, "k", mask);
        assert_eq!(
            urls("12345+octo@users.noreply.github.com", all),
            [
                Lookup::Image("https://avatars.githubusercontent.com/u/12345?s=128".into()),
                Lookup::GitlabJson(
                    "https://gitlab.com/api/v4/avatar?email=12345%2Bocto%40users.noreply.github.com&size=128".into()
                ),
                Lookup::Image("https://www.gravatar.com/avatar/k?s=128&d=404&r=g".into()),
                Lookup::Image("https://seccdn.libravatar.org/avatar/k?s=128&d=404".into()),
            ]
        );
        assert_eq!(
            urls(
                "octo@users.noreply.github.com",
                mask(&[AvatarSource::Github])
            ),
            [Lookup::Image(
                "https://avatars.githubusercontent.com/octo?s=128".into()
            )]
        );
        assert!(
            urls("ada@example.org", mask(&[AvatarSource::Github])).is_empty(),
            "GitHub only covers its no-reply addresses"
        );
        assert_eq!(
            urls(
                "ada@example.org",
                mask(&[AvatarSource::Github, AvatarSource::Gravatar])
            ),
            [Lookup::Image(
                "https://www.gravatar.com/avatar/k?s=128&d=404&r=g".into()
            )]
        );
        assert!(urls("ada@example.org", 0).is_empty());
    }
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
