//! The error type returned by header generation.

use crate::validate::{UnrepresentableTypesError, UnresolvedTypesError};
use std::error::Error;
use std::fmt;
use std::io;
use std::path::PathBuf;

/// Everything that can go wrong while generating a C header.
///
/// Deliberately names no third-party type. `cargo_metadata` and `syn` failures are
/// captured as messages rather than wrapped, so neither becomes part of this
/// crate's public API and a breaking release of either cannot break this one.
#[derive(Debug)]
#[non_exhaustive]
pub enum TerraffiError {
    /// The configured crate directory does not exist, or is not a directory.
    CrateDirNotFound(PathBuf),

    /// `cargo metadata` could not be run for the crate manifest.
    CargoMetadata {
        /// The manifest `cargo metadata` was pointed at.
        manifest_path: PathBuf,
        /// The failure reported by `cargo metadata`.
        message: String,
    },

    /// The manifest describes a workspace rather than a single package.
    NoRootPackage(PathBuf),

    /// A source file could not be read.
    ReadSource {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O failure.
        source: io::Error,
    },

    /// A source file could not be parsed as Rust.
    ParseSource {
        /// The file that could not be parsed.
        path: PathBuf,
        /// The parser's message.
        message: String,
    },

    /// An item could not be translated into a C declaration.
    UnsupportedItem(String),

    /// The header would reference types that it does not declare.
    ///
    /// Returned unless [`crate::TerraffiGeneratorBuilder::allow_unresolved_types`]
    /// is set.
    UnresolvedTypes(UnresolvedTypesError),

    /// A function signature uses a type C cannot express in one declaration.
    ///
    /// Slices and vectors expand into several members as struct fields, which has
    /// no equivalent in a parameter or return type.
    UnrepresentableTypes(UnrepresentableTypesError),

    /// The header could not be written.
    Write(fmt::Error),
}

impl fmt::Display for TerraffiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TerraffiError::CrateDirNotFound(path) => {
                write!(f, "crate directory `{}` is not a directory", path.display())
            }
            TerraffiError::CargoMetadata {
                manifest_path,
                message,
            } => write!(
                f,
                "could not read cargo metadata for `{}`: {message}",
                manifest_path.display()
            ),
            TerraffiError::NoRootPackage(manifest_path) => write!(
                f,
                "could not determine the root package from `{}`. The manifest path must \
                 point at a specific package's Cargo.toml, not a workspace virtual manifest",
                manifest_path.display()
            ),
            TerraffiError::ReadSource { path, source } => {
                write!(f, "could not read `{}`: {source}", path.display())
            }
            TerraffiError::ParseSource { path, message } => {
                write!(f, "could not parse `{}`: {message}", path.display())
            }
            TerraffiError::UnsupportedItem(message) => f.write_str(message),
            TerraffiError::UnresolvedTypes(e) => fmt::Display::fmt(e, f),
            TerraffiError::UnrepresentableTypes(e) => fmt::Display::fmt(e, f),
            TerraffiError::Write(e) => write!(f, "could not write the header: {e}"),
        }
    }
}

impl Error for TerraffiError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            TerraffiError::ReadSource { source, .. } => Some(source),
            TerraffiError::UnresolvedTypes(e) => Some(e),
            TerraffiError::UnrepresentableTypes(e) => Some(e),
            TerraffiError::Write(e) => Some(e),
            _ => None,
        }
    }
}

impl From<UnresolvedTypesError> for TerraffiError {
    fn from(value: UnresolvedTypesError) -> Self {
        TerraffiError::UnresolvedTypes(value)
    }
}

impl From<UnrepresentableTypesError> for TerraffiError {
    fn from(value: UnrepresentableTypesError) -> Self {
        TerraffiError::UnrepresentableTypes(value)
    }
}

impl From<fmt::Error> for TerraffiError {
    fn from(value: fmt::Error) -> Self {
        TerraffiError::Write(value)
    }
}
