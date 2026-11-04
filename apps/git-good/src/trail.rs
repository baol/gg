//! Back and Forward: a browser-style list of the commits and files the user
//! jumped to. A jump (a click, a search result, a ref, opening Review)
//! adds a place after the current one and drops any places forward of it;
//! stepping with the keys moves the current place instead, as scrolling a
//! page does not add to a browser's history. Each place keeps its scroll
//! position, at most [`LIMIT`] are kept, and places whose commit is gone
//! are skipped. Source: designcraft `crates/ui-egui/src/page_history.rs`.

use gg_engine::review::ReviewRequest;

/// The most places kept; the oldest go first.
pub const LIMIT: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub enum Place {
    /// History with a commit selected, its filter, and where it was scrolled.
    History {
        selected: String,
        query: String,
        offset: f32,
    },
    /// Review of a comparison, on a file when one was selected.
    Review {
        request: ReviewRequest,
        subject: String,
        file: Option<String>,
    },
}

impl Place {
    /// Whether `other` is the same place, wherever it was scrolled to.
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::History {
                    selected, query, ..
                },
                Self::History {
                    selected: other_selected,
                    query: other_query,
                    ..
                },
            ) => selected == other_selected && query == other_query,
            (
                Self::Review { request, file, .. },
                Self::Review {
                    request: other_request,
                    file: other_file,
                    ..
                },
            ) => request == other_request && file == other_file,
            _ => false,
        }
    }

    /// The commit this place shows, to tell whether it is gone.
    pub fn commit(&self) -> &str {
        match self {
            Self::History { selected, .. } => selected,
            Self::Review { request, .. } => &request.revision,
        }
    }
}

#[derive(Debug, Default)]
pub struct Trail {
    places: Vec<Place>,
    /// The current place, when there is one.
    at: Option<usize>,
}

impl Trail {
    /// The user jumped to `place`: it follows the current place, and the
    /// places forward of it are dropped. The same place again only keeps
    /// its new scroll position.
    pub fn visit(&mut self, place: Place) {
        if let Some(at) = self.at {
            if self.places[at].same(&place) {
                self.places[at] = place;
                return;
            }
            self.places.truncate(at + 1);
        }
        self.places.push(place);
        if self.places.len() > LIMIT {
            self.places.remove(0);
        }
        self.at = Some(self.places.len() - 1);
    }

    /// The user stepped to `place` from the current one: it replaces the
    /// current place, and Back and Forward keep their places.
    pub fn follow(&mut self, place: Place) {
        match self.at {
            Some(at) => self.places[at] = place,
            None => self.visit(place),
        }
    }

    pub fn can_back(&self) -> bool {
        self.at.is_some_and(|at| at > 0)
    }

    pub fn can_forward(&self) -> bool {
        self.at.is_some_and(|at| at + 1 < self.places.len())
    }

    /// Go back past places `gone` says no longer exist (and drop them);
    /// the place to show, if any is left.
    pub fn back(&mut self, gone: impl Fn(&Place) -> bool) -> Option<Place> {
        self.step(-1, gone)
    }

    /// Go forward, as [`Trail::back`].
    pub fn forward(&mut self, gone: impl Fn(&Place) -> bool) -> Option<Place> {
        self.step(1, gone)
    }

    fn step(&mut self, direction: isize, gone: impl Fn(&Place) -> bool) -> Option<Place> {
        let mut at = self.at?;
        loop {
            let next = at.checked_add_signed(direction)?;
            if next >= self.places.len() {
                return None;
            }
            if gone(&self.places[next]) {
                self.places.remove(next);
                // Going back, the current place moved down with the removal.
                if direction < 0 {
                    at -= 1;
                }
                self.at = Some(at);
                continue;
            }
            self.at = Some(next);
            return Some(self.places[next].clone());
        }
    }

    /// Forget every place, as when another repository opens.
    pub fn clear(&mut self) {
        self.places.clear();
        self.at = None;
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.places.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(commit: &str, offset: f32) -> Place {
        Place::History {
            selected: commit.into(),
            query: String::new(),
            offset,
        }
    }

    #[test]
    fn back_and_forward_walk_the_places_jumped_to() {
        let mut trail = Trail::default();
        assert!(!trail.can_back() && !trail.can_forward());
        trail.visit(at("a", 0.0));
        trail.visit(at("b", 10.0));
        trail.visit(at("c", 20.0));
        // Scrolling on the same place keeps one entry with the new offset.
        trail.visit(at("c", 25.0));
        assert_eq!(trail.len(), 3);
        assert_eq!(trail.back(|_| false), Some(at("b", 10.0)));
        assert_eq!(trail.back(|_| false), Some(at("a", 0.0)));
        assert_eq!(trail.back(|_| false), None, "nothing before the first");
        assert_eq!(trail.forward(|_| false), Some(at("b", 10.0)));
        // A new jump drops what was forward.
        trail.visit(at("d", 0.0));
        assert!(!trail.can_forward());
        assert_eq!(trail.back(|_| false), Some(at("b", 10.0)));
    }

    #[test]
    fn stepping_moves_the_current_place_without_adding_one() {
        let mut trail = Trail::default();
        trail.visit(at("a", 0.0));
        trail.visit(at("b", 0.0));
        trail.follow(at("c", 0.0));
        assert_eq!(trail.len(), 2);
        assert_eq!(trail.back(|_| false), Some(at("a", 0.0)));
        assert_eq!(trail.forward(|_| false), Some(at("c", 0.0)));
    }

    #[test]
    fn gone_places_are_skipped_and_dropped() {
        let mut trail = Trail::default();
        for commit in ["a", "gone", "b", "gone", "c"] {
            trail.visit(at(commit, 0.0));
        }
        let gone = |place: &Place| place.commit() == "gone";
        assert_eq!(trail.back(gone), Some(at("b", 0.0)));
        assert_eq!(trail.back(gone), Some(at("a", 0.0)));
        assert_eq!(trail.len(), 3, "gone places are dropped");
        assert_eq!(trail.forward(gone), Some(at("b", 0.0)));
        assert_eq!(trail.forward(gone), Some(at("c", 0.0)));
    }

    #[test]
    fn at_most_the_limit_is_kept() {
        let mut trail = Trail::default();
        for n in 0..LIMIT + 10 {
            trail.visit(at(&n.to_string(), 0.0));
        }
        assert_eq!(trail.len(), LIMIT);
        let mut oldest = None;
        while let Some(place) = trail.back(|_| false) {
            oldest = Some(place);
        }
        assert_eq!(oldest, Some(at("10", 0.0)), "the oldest went first");
    }
}
