//! Kitchen: pantry items, harness symlinks, stove, and chat.

pub mod chat;
pub mod config;
pub mod item;
pub mod ops;
pub mod stove;

pub use config::{load_harnesses, repo_root, Harness, KindSpec};
pub use item::{classify, items_for, Item, LinkState};
pub use ops::{link_item, unlink_item, ActionResult};
