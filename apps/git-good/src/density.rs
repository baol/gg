//! Row density: compact (the default) or comfortable.
//!
//! The density is one process-wide value so fixed-height list rows can read it
//! without threading a context through every geometry calculation. Tests keep
//! the compact default.

use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};

static COMFORTABLE: AtomicBool = AtomicBool::new(false);
/// Code font size in tenths of a pixel.
static CODE_FONT: AtomicU16 = AtomicU16::new(125);

pub fn set_code_font(tenths: u16) {
    CODE_FONT.store(tenths, Ordering::Relaxed);
}

/// The reader's code font size, from Preferences › Appearance.
pub fn code_font() -> f32 {
    f32::from(CODE_FONT.load(Ordering::Relaxed)) / 10.0
}

/// List row height in compact density.
pub const COMPACT_ROW: f32 = 22.0;
/// List row height in comfortable density.
pub const COMFORTABLE_ROW: f32 = 28.0;

pub fn set(density: crate::settings::Density) {
    COMFORTABLE.store(
        density == crate::settings::Density::Comfortable,
        Ordering::Relaxed,
    );
}

/// Height of history, file, and refs rows.
pub fn row() -> f32 {
    row_for(if COMFORTABLE.load(Ordering::Relaxed) {
        crate::settings::Density::Comfortable
    } else {
        crate::settings::Density::Compact
    })
}

pub fn row_for(density: crate::settings::Density) -> f32 {
    match density {
        crate::settings::Density::Compact => COMPACT_ROW,
        crate::settings::Density::Comfortable => COMFORTABLE_ROW,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn densities_name_their_row_heights() {
        use crate::settings::Density;
        assert_eq!(super::row_for(Density::Compact), 22.0);
        assert_eq!(super::row_for(Density::Comfortable), 28.0);
        assert_eq!(super::row(), 22.0, "tests run in compact density");
    }
}
