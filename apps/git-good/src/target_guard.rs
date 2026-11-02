//! Rows may change under the pointer; a click never lands on a different
//! target than the one the user aimed at.
//!
//! Lists show new content as soon as it arrives. When the row under the
//! pointer changes identity (a refresh, a page arriving, a filter result),
//! that row fades in over [`FADE`] seconds and refuses clicks for [`GRACE`]
//! seconds, so a click already on its way is dropped instead of acting on
//! the new row. A click also counts only on the row that was pressed.
//!
//! The user's own motion (moving the pointer, scrolling, dragging) is not a
//! change: only a different row under a pointer that stayed put starts the
//! grace.

/// How long a row that changed under the pointer refuses clicks: just above
/// human reaction time.
pub const GRACE: f64 = 0.3;
/// How long its new content takes to fade in.
pub const FADE: f64 = 0.25;

/// What the hovered row may do this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Target {
    /// Opacity for the row's content, 0 to 1.
    pub alpha: f32,
    /// Whether a click released on this row may act.
    pub clickable: bool,
}

impl Target {
    pub const STILL: Self = Self {
        alpha: 1.0,
        clickable: true,
    };
}

/// One per list. Call [`Guard::hover`] for the row under the pointer each
/// frame, before acting on its click.
#[derive(Debug)]
pub struct Guard<K> {
    /// What the pointer was over last frame, and where the pointer was.
    under: Option<K>,
    point: Option<eframe::egui::Pos2>,
    /// When it last changed under a still pointer.
    changed: f64,
    /// What was under the pointer when the button went down.
    press: Press<K>,
}

#[derive(Debug)]
enum Press<K> {
    Up,
    On(K),
    /// The row swapped while the button was down: no click until release.
    Void,
}

impl<K> Default for Guard<K> {
    fn default() -> Self {
        Self {
            under: None,
            point: None,
            changed: f64::NEG_INFINITY,
            press: Press::Up,
        }
    }
}

impl<K: PartialEq + Clone> Guard<K> {
    /// The row showing `key` is under the pointer at `point` at `now`.
    /// `acted` is true when the user scrolled, dragged or pressed a reading
    /// key this frame; a pointer at a new position counts as the user's own
    /// motion too, compared here rather than by egui's delta, which reads
    /// zero when the pointer comes back after leaving the window. `pressed`
    /// is true on the frame the primary button went down.
    pub fn hover(
        &mut self,
        now: f64,
        key: &K,
        point: eframe::egui::Pos2,
        acted: bool,
        pressed: bool,
    ) -> Target {
        let moved = acted || self.point.replace(point) != Some(point);
        match &self.under {
            Some(old) if old != key && !moved => {
                self.changed = now;
                // The press was aimed at what was there before.
                if matches!(self.press, Press::On(_)) {
                    self.press = Press::Void;
                }
            }
            Some(old) if old == key => {}
            _ => self.changed = f64::NEG_INFINITY,
        }
        self.under = Some(key.clone());
        if pressed {
            self.press = Press::On(key.clone());
        }
        let age = now - self.changed;
        let aimed = match &self.press {
            Press::Up => true,
            Press::On(down) => down == key,
            Press::Void => false,
        };
        Target {
            alpha: (age / FADE).clamp(0.0, 1.0) as f32,
            clickable: age >= GRACE && aimed,
        }
    }

    /// Nothing of this list is under the pointer this frame.
    pub fn leave(&mut self) {
        self.under = None;
        self.point = None;
        self.changed = f64::NEG_INFINITY;
        self.press = Press::Up;
    }

    /// The button was released: the next press starts fresh.
    pub fn release(&mut self) {
        self.press = Press::Up;
    }

    /// Whether a fade or grace is still running and needs another frame.
    pub fn animating(&self, now: f64) -> bool {
        now - self.changed < GRACE
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const HERE: eframe::egui::Pos2 = eframe::egui::pos2(10.0, 10.0);
    const THERE: eframe::egui::Pos2 = eframe::egui::pos2(10.0, 40.0);

    #[test]
    fn a_resting_row_is_still_and_clickable() {
        let mut guard = Guard::default();
        assert_eq!(guard.hover(0.0, &"a", THERE, false, false), Target::STILL);
        assert_eq!(guard.hover(1.0, &"a", HERE, false, false), Target::STILL);
    }

    #[test]
    fn moving_or_scrolling_onto_another_row_is_not_a_change() {
        let mut guard = Guard::default();
        guard.hover(0.0, &"a", HERE, false, false);
        assert_eq!(guard.hover(0.01, &"b", THERE, false, false), Target::STILL);
    }

    #[test]
    fn a_row_that_changes_under_a_still_pointer_fades_in_and_refuses_clicks_briefly() {
        let mut guard = Guard::default();
        guard.hover(0.0, &"a", HERE, false, false);
        let swapped = guard.hover(1.0, &"b", HERE, false, false);
        assert_eq!(swapped.alpha, 0.0);
        assert!(!swapped.clickable);
        let half = guard.hover(1.0 + FADE / 2.0, &"b", HERE, false, false);
        assert!(half.alpha > 0.4 && half.alpha < 0.6);
        assert!(!half.clickable);
        assert!(guard.animating(1.0 + FADE));
        assert_eq!(
            guard.hover(1.0 + GRACE, &"b", HERE, false, false),
            Target::STILL
        );
        assert!(!guard.animating(1.0 + GRACE));
    }

    #[test]
    fn a_click_counts_only_on_the_row_that_was_pressed() {
        let mut guard = Guard::default();
        guard.hover(0.0, &"a", HERE, false, true);
        // The row swapped between press and release: that press is void,
        // even once the grace has passed.
        assert!(!guard.hover(0.1, &"b", HERE, false, false).clickable);
        assert!(!guard.hover(0.1 + GRACE, &"b", HERE, false, false).clickable);
        guard.release();
        guard.hover(1.0, &"b", HERE, false, true);
        assert!(guard.hover(1.05, &"b", HERE, false, false).clickable);
    }

    #[test]
    fn leaving_clears_the_grace_and_the_press() {
        let mut guard = Guard::default();
        guard.hover(0.0, &"a", HERE, false, true);
        guard.hover(0.1, &"b", HERE, false, false);
        guard.leave();
        assert_eq!(guard.hover(0.15, &"b", THERE, false, false), Target::STILL);
    }

    #[test]
    fn coming_back_onto_another_row_after_leaving_the_window_is_not_a_change() {
        let mut guard = Guard::default();
        guard.hover(0.0, &"a", HERE, false, false);
        // egui reports no delta here; the position still moved.
        assert_eq!(guard.hover(0.01, &"b", THERE, false, false), Target::STILL);
        // A scroll under a still pointer is the user's motion as well.
        assert_eq!(guard.hover(0.02, &"c", THERE, true, false), Target::STILL);
    }
}
