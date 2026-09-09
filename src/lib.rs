//! Kitchen: pantry items, harness symlinks, stove, and chat.

pub mod bootstrap;
pub mod chat;
pub mod config;
pub mod item;
pub mod ops;
pub mod stove;

pub use bootstrap::{bootstrap, BOOTSTRAP_SKILLS};
pub use config::{load_harnesses, repo_root, Harness, KindSpec};
pub use item::{classify, items_for, list_pantry, Item, LinkState, PantryEntry};
pub use ops::{link_item, unlink_item, ActionResult};
