//! Admin config data models and service.
//!
//! Ported from `src/harsh_realm/models/admin/` and
//! `src/harsh_realm/admin/`. The Python `RowBackedModel`
//! base and its `from_row` constructors are dropped — deserialization happens
//! via serde once the persistence layer is ported.

pub mod base;
pub mod common;
pub mod config;
pub mod faction;
pub mod oracle_dungeon;
pub mod service;
pub mod transfer;

pub use service::AdminService;
