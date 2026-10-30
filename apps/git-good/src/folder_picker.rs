//! Native folder selection for path fields.

use std::path::{Path, PathBuf};

/// Ask the desktop for a folder, starting at `start` when it still exists.
pub fn pick_folder(title: &str, start: Option<&Path>) -> Option<PathBuf> {
    let mut dialog = rfd::FileDialog::new().set_title(title);
    if let Some(start) = start_directory(start) {
        dialog = dialog.set_directory(start);
    }
    dialog.pick_folder()
}

fn start_directory(start: Option<&Path>) -> Option<PathBuf> {
    start.filter(|path| path.is_dir()).map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_existing_folders_seed_the_dialog() -> Result<(), Box<dyn std::error::Error>> {
        let folder = tempfile::tempdir()?;
        assert_eq!(
            start_directory(Some(folder.path())),
            Some(folder.path().to_path_buf())
        );
        let missing = folder.path().join("missing");
        assert_eq!(start_directory(Some(&missing)), None);
        assert_eq!(start_directory(None), None);
        let file = folder.path().join("file");
        std::fs::write(&file, b"x")?;
        assert_eq!(start_directory(Some(&file)), None);
        Ok(())
    }
}
