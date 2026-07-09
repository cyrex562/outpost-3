//! Editor data-access helpers.
//!
//! Ported from `src/harsh_realm/api/editor/`. HTTP route declarations
//! (`@router.*`) belong to the B9 web-host layer (future Axum server) and are
//! not included here.

pub mod cells;
pub mod transfer;

pub use cells::EditorCellRepository;
pub use transfer::{export_query, is_exportable, EXPORT_TABLES};
