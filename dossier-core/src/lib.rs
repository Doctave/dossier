use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::Utf8Error,
};

use serde::Serialize;
use thiserror::Error;

pub use tree_sitter;

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
pub struct Entity {
    /// The title for the entity. Usually the name of the class/function/module, etc.
    pub title: String,
    /// A description for the entity. Supports Markdown.
    pub description: MarkdownString,
    /// The type of the entity. E.g. function, class, module.
    /// Each language will have a different set of entities.
    pub kind: String,
    /// Child entities. E.g. classes may contain functions, modules may have child modules, etc.
    pub children: Vec<Entity>,
    /// The language of the entity
    pub language: String,
    /// The language of the entity
    pub source: Source,
}

#[derive(Debug, Clone, Serialize)]
/// Metadata about the source of an `Entity`
pub struct Source {
    pub file: PathBuf,
    /// Starting offset of the entity in the source file in bytes
    pub start_offset_bytes: usize,
    /// Ending offset of the entity in the source file in bytes
    pub end_offset_bytes: usize,
    /// Optional: Git repository URL for the file
    pub repository: Option<String>,
}

#[derive(Debug, Clone)]
/// Metadata about the source of an `Entity`
pub struct Config {}

/// The trait for implementing language-specific parsers
pub trait DocsParser {
    /// Given a pathname to an entry point, return a list of entities
    fn parse(&self, path: &Path, config: &Config) -> Result<Vec<Entity>>;
}

pub mod helpers {
    use tree_sitter::QueryCapture;

    use crate::DossierError;

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
