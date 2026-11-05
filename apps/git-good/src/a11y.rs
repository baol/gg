//! Keyboard reach for the window's regions (with AccessKit on, screen
//! readers see the widgets egui describes). F6 moves the keyboard to the
//! next region and Shift+F6 to the previous one: the toolbar's search
//! field, the docked refs sidebar's filter, and the content (the history
//! or changes list, whose keys work while no field has focus). A region
//! that is not shown is skipped. Shift+F10 and the Menu key open the
//! context menu of what is selected. Source: figcraft
//! `crates/ui-egui/src/a11y.rs`.

use eframe::egui;

/// A region F6 moves between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Region {
    Search,
    Refs,
    Content,
}

/// The fields that stand for each region, when they are shown this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Regions {
    pub search: Option<egui::Id>,
    pub refs: Option<egui::Id>,
}

impl Regions {
    /// The regions shown, in F6 order.
    fn order(&self) -> Vec<Region> {
        let mut order = Vec::with_capacity(3);
        if self.search.is_some() {
            order.push(Region::Search);
        }
        if self.refs.is_some() {
            order.push(Region::Refs);
        }
        order.push(Region::Content);
        order
    }

    /// The region the keyboard is in: the one whose field has focus, else
    /// the content.
    pub fn current(&self, focused: Option<egui::Id>) -> Region {
        match focused {
            Some(id) if Some(id) == self.search => Region::Search,
            Some(id) if Some(id) == self.refs => Region::Refs,
            _ => Region::Content,
        }
    }

    /// The region after (or before) `from`, wrapping.
    pub fn next(&self, from: Region, backwards: bool) -> Region {
        let order = self.order();
        let at = order.iter().position(|region| *region == from).unwrap_or(0);
        let len = order.len();
        let step = if backwards { len - 1 } else { 1 };
        order[(at + step) % len]
    }

    /// Move the keyboard to `region`.
    fn enter(&self, ctx: &egui::Context, region: Region) {
        match region {
            Region::Search | Region::Refs => {
                let id = if region == Region::Search {
                    self.search
                } else {
                    self.refs
                };
                if let Some(id) = id {
                    ctx.memory_mut(|memory| memory.request_focus(id));
                }
            }
            Region::Content => crate::navigation::focus_content(ctx),
        }
    }
}

/// F6 and Shift+F6, while no dialog, menu or popup holds the keyboard: the
/// region moved to.
pub fn cycle(ctx: &egui::Context, regions: &Regions) -> Option<Region> {
    if egui::Popup::is_any_open(ctx)
        || crate::menu_keys::is_focused(ctx)
        || ctx.memory(|memory| memory.top_modal_layer()).is_some()
    {
        return None;
    }
    let backwards = ctx.input_mut(|input| input.consume_key(egui::Modifiers::SHIFT, egui::Key::F6));
    let forwards = !backwards
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F6));
    if !backwards && !forwards {
        return None;
    }
    let from = regions.current(ctx.memory(|memory| memory.focused()));
    let to = regions.next(from, backwards);
    regions.enter(ctx, to);
    Some(to)
}

/// Shift+F10 or the Menu key: open the selected item's context menu.
pub fn context_menu_key(ctx: &egui::Context) -> bool {
    ctx.input_mut(|input| {
        input.consume_key(egui::Modifiers::SHIFT, egui::Key::F10)
            || input.consume_key(egui::Modifiers::NONE, egui::Key::Menu)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn f6_walks_the_shown_regions_and_wraps() {
        let search = egui::Id::unique("search");
        let refs = egui::Id::unique("refs");
        let all = Regions {
            search: Some(search),
            refs: Some(refs),
        };
        assert_eq!(all.current(None), Region::Content);
        assert_eq!(all.current(Some(refs)), Region::Refs);
        assert_eq!(all.next(Region::Content, false), Region::Search);
        assert_eq!(all.next(Region::Search, false), Region::Refs);
        assert_eq!(all.next(Region::Refs, false), Region::Content);
        assert_eq!(all.next(Region::Search, true), Region::Content);
        // No docked sidebar: it is skipped.
        let narrow = Regions {
            search: Some(search),
            refs: None,
        };
        assert_eq!(narrow.next(Region::Search, false), Region::Content);
        assert_eq!(narrow.next(Region::Content, true), Region::Search);
        // Only the content: F6 stays there.
        assert_eq!(
            Regions::default().next(Region::Content, false),
            Region::Content
        );
    }
}
