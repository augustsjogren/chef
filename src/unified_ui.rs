use anyhow::Result;

use crate::pantry_ui;

/// Unified pantry TUI. Serve is a first-class navigation kind.
pub fn run() -> Result<()> {
    pantry_ui::run()
}
