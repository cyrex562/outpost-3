//! Content loading error types with file and field context.

use thiserror::Error;

/// All errors that can occur while loading or validating a content pack.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum ContentError {
    /// A required field was absent in a record.
    #[error("{file}: record '{id}' is missing required field '{field}'")]
    MissingField {
        /// Source file name.
        file: String,
        /// Record id (or index if id is absent).
        id: String,
        /// Name of the missing field.
        field: String,
    },

    /// Two records share the same ID within the same content table.
    #[error("{file}: duplicate id '{id}' (previously defined in '{prior_file}')")]
    DuplicateId {
        /// File where the duplicate was found.
        file: String,
        /// The conflicting id.
        id: String,
        /// File where the id was first defined.
        prior_file: String,
    },

    /// A field contained a value that did not match any known enum variant.
    #[error("{file}: record '{id}' field '{field}' has unknown value '{value}'")]
    UnknownEnumValue {
        /// Source file name.
        file: String,
        /// Record id.
        id: String,
        /// Field containing the bad value.
        field: String,
        /// The unrecognised value string.
        value: String,
    },

    /// The YAML in a file could not be parsed.
    #[error("{file}: YAML parse error: {message}")]
    ParseError {
        /// Source file name.
        file: String,
        /// Human-readable error from the YAML parser.
        message: String,
    },

    /// The pack manifest (`pack.yaml`) was not supplied.
    #[error("pack manifest (pack.yaml) is missing")]
    MissingManifest,

    /// A recipe references a commodity id that has not been defined.
    #[error("{file}: recipe '{id}' references unknown commodity '{commodity_id}'")]
    UnknownCommodityRef {
        /// Source file name.
        file: String,
        /// Recipe id.
        id: String,
        /// The unresolved commodity id.
        commodity_id: String,
    },

    /// A `SystemBodyDef`'s `subtype` isn't valid for its `kind` (issue #196).
    #[error(
        "{file}: system '{system_id}' body '{body_name}' has subtype '{subtype:?}', \
         which is not valid for body kind '{kind:?}'"
    )]
    IncompatiblePlanetarySubtype {
        /// Source file name.
        file: String,
        /// Star system id.
        system_id: String,
        /// Body display name.
        body_name: String,
        /// The body kind involved.
        kind: crate::system::BodyKind,
        /// The rejected subtype.
        subtype: crate::system::PlanetarySubtype,
    },

    /// A `SystemBodyDef`'s `parent_body` doesn't name another body in the
    /// same star system (issue #196).
    #[error(
        "{file}: system '{system_id}' body '{body_name}' has parent_body \
         '{parent_name}', which does not match any other body in the same system"
    )]
    UnknownParentBodyRef {
        /// Source file name.
        file: String,
        /// Star system id.
        system_id: String,
        /// Body display name.
        body_name: String,
        /// The unresolved parent body name.
        parent_name: String,
    },
}
