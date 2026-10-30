//! System clipboard access for prefilling clone fields.
use gg_engine::sync;

/// Read the clipboard text, returning nothing when no clipboard service is available.
pub fn read_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// Read a repository URL from the clipboard, ignoring unrelated text.
pub fn repository_url() -> Option<String> {
    read_text().as_deref().and_then(sync::repository_url)
}
