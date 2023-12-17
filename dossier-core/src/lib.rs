use std::path::{Path, PathBuf};

use thiserror::Error;

pub type Result<T> = std::result::Result<T, DossierError>;

#[derive(Error, Debug)]
pub enum DossierError {}

pub type MarkdownString = String;

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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
