//! The right-click menu of a file row in Review and in the commit details:
//! the file actions that need only the row (REV-82).
use eframe::egui;

/// A choice in a file row's right-click menu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Read the file's changes in the reader.
    Read,
    /// The file's versions, as H opens them.
    History,
    /// Open in editor (`false`) or Reveal in folder (`true`).
    Open {
        folder: bool,
    },
    CopyPath,
}

/// Draw the menu's rows; the chosen one, if any, closes it.
pub fn show(ui: &mut egui::Ui) -> Option<Choice> {
    use crate::menus::row;
    ui.set_min_width(220.0);
    let mut choice = None;
    if ui
        .add(row(false, "Read file").shortcut_text("Enter"))
        .clicked()
    {
        choice = Some(Choice::Read);
    }
    if ui
        .add(row(false, "File history").shortcut_text("H"))
        .clicked()
    {
        choice = Some(Choice::History);
    }
    ui.separator();
    if ui
        .add(row(false, "Open in editor"))
        .on_hover_text("Open the working file with the editor from Preferences")
        .clicked()
    {
        choice = Some(Choice::Open { folder: false });
    }
    if ui
        .add(row(false, "Reveal in folder"))
        .on_hover_text("Open the folder that holds the working file")
        .clicked()
    {
        choice = Some(Choice::Open { folder: true });
    }
    if ui.add(row(false, "Copy path")).clicked() {
        choice = Some(Choice::CopyPath);
    }
    if choice.is_some() {
        ui.close();
    }
    choice
}
