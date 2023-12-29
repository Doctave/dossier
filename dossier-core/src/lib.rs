use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::Utf8Error,
};

use serde::Serialize;
use thiserror::Error;

pub use indexmap;
pub use tree_sitter;
pub use serde_json;

pub type Result<T> = std::result::Result<T, DossierError>;

#[derive(Error, Debug)]
pub enum DossierError {
    UTF8Error(Utf8Error),
}

impl Display for DossierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use DossierError::*;
        match &self {
            UTF8Error(error) => {
                write!(f, "UTF8Error: {:?}", error)
            }
        }
    }
}

pub type MarkdownString = String;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entity {
    /// The title for the entity. Usually the name of the class/function/module, etc.
    pub title: String,
    /// A description for the entity. Supports Markdown.
    pub description: MarkdownString,
    /// The type of the entity. E.g. function, class, module.
    /// Each language will have a different set of entities.
    pub kind: String,
    /// Child entities. E.g. classes may contain functions, modules may have child modules, etc.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<Entity>,
    /// What context the entity is in. E.g. a type may be describing a parameter to a function, or a return type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_context: Option<String>,
    /// The language of the entity
    pub language: String,
    /// The language of the entity
    pub source: Source,
    /// Arbitrary metadata different types of entities need to store
    #[serde(skip_serializing_if = "value_is_empty")]
    pub meta: serde_json::Value,
}

fn value_is_empty(value: &serde_json::Value) -> bool {
    value.is_null() || value.as_object().map(|o| o.is_empty()).unwrap_or(false)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
/// Metadata about the source of an `Entity`
pub struct Source {
    pub file: PathBuf,
    /// Starting offset of the entity in the source file in bytes
    pub start_offset_bytes: usize,
    /// Ending offset of the entity in the source file in bytes
    pub end_offset_bytes: usize,
    /// Optional: Git repository URL for the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone)]
/// A config passed into parsers.
///
/// Placeholder for now, but in the future could contain information
/// about the parsing context like the current repository, etc.
pub struct Config {}

/// The trait for implementing language-specific parsers
pub trait DocsParser {
    /// Given a pathname to an entry point, return a list of entities
    fn parse(&self, path: &Path, config: &Config) -> Result<Vec<Entity>>;
}

pub mod helpers {
    use tree_sitter::{QueryCapture, Query, Node};

    use crate::DossierError;

    pub fn node_for_capture<'a>(name: &str, captures: &'a [QueryCapture<'a>], query: &Query) -> Option<Node<'a>> {
        query
            .capture_index_for_name(name)
            .and_then(|index| captures.iter().find(|c| c.index == index))
            .map(|c| c.node)
    }

    pub fn get_string_from_match<'a>(
        captures: &'a [QueryCapture],
        index: u32,
        code: &'a str,
    ) -> Option<crate::Result<&'a str>> {
        captures.iter().find(|c| c.index == index).map(|capture| {
            capture
                .node
                .utf8_text(code.as_bytes())
                .map_err(DossierError::UTF8Error)
        })
    }
}
