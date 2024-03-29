use std::{
    fmt::Display,
    path::{Path, PathBuf},
    str::Utf8Error,
};

use serde::Serialize;
use thiserror::Error;

pub use indexmap;
pub use serde_json;
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
pub type FullyQualifiedName = String;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum Identity {
    /// The fully qualified name of an entity
    #[serde(rename = "fqn")]
    FQN(FullyQualifiedName),
    /// A reference to another entity via its fully qualified name
    #[serde(rename = "refers_to")]
    Reference(FullyQualifiedName),
    #[serde(skip_serializing)]
    Anonymous,
}

impl Identity {
    pub fn is_anonymous(&self) -> bool {
        matches!(self, Identity::Anonymous)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Entity {
    /// The title for the entity. Usually the name of the class/function/module, etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// A description for the entity. Supports Markdown.
    pub description: MarkdownString,
    /// The type of the entity. E.g. function, class, module.
    /// Each language will have a different set of entities.
    pub kind: String,
    /// The identity of the entity: either its fully qualified name, or a reference to another entity
    /// via its fully qualified name.
    ///
    /// E.g. a class declaration will have an identity of its fully qualified name, but a
    /// function's return position will have an reference to another entity that describes its type.
    #[serde(flatten)]
    #[serde(skip_serializing_if = "Identity::is_anonymous")]
    pub identity: Identity,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
/// Position in a source file.
///
/// Contains the row and column number, as well as the byte offset from the start of the file,
/// since different situations may call for one of the other.
pub struct Position {
    /// The line number of the entity in the source file
    pub row: usize,
    /// The column number on the line
    pub column: usize,
    /// Byte offset from the start of the file for the entity
    pub byte_offset: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
/// Metadata about the source of an `Entity`
pub struct Source {
    pub file: PathBuf,
    /// Start position in the source file
    pub start: Position,
    /// End position in the source file
    pub end: Position,
    /// Optional: Git repository URL for the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repository: Option<String>,
}

#[derive(Debug, Clone, Default)]
/// A config passed into parsers.
///
/// Placeholder for now, but in the future could contain information
/// about the parsing context like the current repository, etc.
pub struct Context {
    namespace: Vec<String>,
}

impl<'a> Context {
    pub fn new() -> Self {
        Self { namespace: vec![] }
    }

    /// Generates a fully qualified name (FQN) from a path, the current namespace,
    /// and a list of parts
    ///
    /// For example, a file src/foo/bar.ts and parts of [Interface, methodName]
    /// would yield a FQN of `src/foo.bar/ts::Interface::methodName`
    ///
    /// This function is operating-system independent, and will always use `/` as
    /// the path separator.
    pub fn generate_fqn<T>(&self, path: &Path, parts: T) -> String
    where
        T: IntoIterator<Item = &'a str>,
    {
        let mut fqn = format!("{}", path.display()).replace('\\', "/");

        for part in &self.namespace {
            fqn.push_str(&format!("::{}", part));
        }

        for part in parts {
            fqn.push_str(&format!("::{}", part));
        }

        fqn
    }

    pub fn push_namespace(&mut self, namespace: &str) {
        self.namespace.push(namespace.to_owned());
    }

    pub fn pop_namespace(&mut self) {
        self.namespace.pop();
    }
}

/// The trait for implementing language-specific parsers
pub trait DocsParser {
    /// Given a pathname to an entry point, return a list of entities
    fn parse<'a, P: Into<&'a Path>, T: IntoIterator<Item = P>>(
        &self,
        paths: T,
        ctx: &mut Context,
    ) -> Result<Vec<Entity>>;
}

pub trait FileSource {
    fn read_file<'a, P: Into<&'a Path>>(&self, path: P) -> std::io::Result<String>;
}

pub struct FileSystem;

impl FileSource for FileSystem {
    fn read_file<'a, P: Into<&'a Path>>(&self, path: P) -> std::io::Result<String> {
        std::fs::read_to_string(path.into())
    }
}

pub struct InMemoryFileSystem {
    pub files: indexmap::IndexMap<PathBuf, String>,
}

impl FileSource for InMemoryFileSystem {
    fn read_file<'a, P: Into<&'a Path>>(&self, path: P) -> std::io::Result<String> {
        let path: &Path = path.into();
        self.files
            .get(path)
            .map(|s| s.to_owned())
            .ok_or(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("File not found: {}", path.display()),
            ))
    }
}

pub mod helpers {
    use super::*;
    use tree_sitter::{Node, Query, QueryCapture};

    pub fn node_for_capture<'a>(
        name: &str,
        captures: &'a [QueryCapture<'a>],
        query: &Query,
    ) -> Option<Node<'a>> {
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
