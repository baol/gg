//! Exact caller state restored when a repository tool closes.
//!
//! Presentation changes inside a tool never create a location. Recovery adds
//! its own source variant here when it is implemented; do not invent one.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HistoryLocation {
    pub selected: Option<String>,
    pub query: String,
    pub offset: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReturnLocation {
    History(HistoryLocation),
    Details(HistoryLocation),
}

impl ReturnLocation {
    pub fn location(&self) -> &HistoryLocation {
        match self {
            Self::History(location) | Self::Details(location) => location,
        }
    }

    pub fn shows_details(&self) -> bool {
        matches!(self, Self::Details(_))
    }
}
