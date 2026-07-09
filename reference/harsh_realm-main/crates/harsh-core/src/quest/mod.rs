//! Quest content and runtime subsystem (HR-19).
//!
//! A durable subsystem following the `status_effects` template: quests are
//! authored content (`QuestCatalog`), quest instances live in the
//! `quest_instances` table (`QuestRepository`), and `QuestService` owns the
//! accept → progress → complete/fail lifecycle. Domain-event handlers turn
//! service results into client-facing `quest.*` events.

pub mod catalog;
pub mod handlers;
pub mod models;
pub mod repository;
pub mod schema;
pub mod service;

pub use catalog::{QuestCatalog, QuestContent};
pub use handlers::{
    handle_accept_requested, handle_complete_requested, handle_fail_requested,
    handle_progress_requested,
};
pub use models::ActiveQuest;
pub use repository::QuestRepository;
pub use schema::{Quest, QuestObjective};
pub use service::{QuestService, WorldClock};
