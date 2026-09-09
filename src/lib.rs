//! Kitchen: pantry items, harness symlinks, stove, and chat.

pub mod bootstrap;
pub mod chat;
pub mod config;
pub mod import;
pub mod item;
pub mod ops;
pub mod stove;

pub use bootstrap::{bootstrap, BOOTSTRAP_SKILLS};
pub use config::{load_harnesses, pantry_root, repo_root, state_dir, tool_root, Harness, KindSpec};
pub use import::{find_external_skill, import_skill, ImportResult};
pub use item::{classify, items_for, list_pantry, Item, LinkState, PantryEntry};
pub use ops::{link_item, unlink_item, ActionResult};
