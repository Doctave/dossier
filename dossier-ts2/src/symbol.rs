use std::path::PathBuf;

use dossier_core::Entity;

#[derive(Debug, Clone, PartialEq)]
/// A symbol we've discovered in the source code.
pub(crate) struct Symbol {
    pub kind: SymbolKind,
    pub source: Source,
    pub fqn: String,
}

impl Symbol {
    pub fn as_entity(&self) -> Entity {
        match &self.kind {
            SymbolKind::Function(f) => f.as_entity(&self.source, &self.fqn),
            SymbolKind::TypeAlias(a) => a.as_entity(&self.source, &self.fqn),
            SymbolKind::Type(t) => t.as_entity(&self.source, &self.fqn),
        }
    }

    pub fn identifier(&self) -> &str {
        match &self.kind {
            SymbolKind::Function(f) => f.identifier.as_str(),
            SymbolKind::TypeAlias(a) => a.identifier.as_str(),
            SymbolKind::Type(t) => t.identifier(),
        }
    }
}

/// The type of the symbol.
/// Contains all the metadata associated with that type of symbol
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Function(crate::function::Function),
    TypeAlias(crate::type_alias::TypeAlias),
    Type(crate::types::Type),
}

impl SymbolKind {
    #[cfg(test)]
    pub fn as_function(&self) -> Option<&crate::function::Function> {
        match self {
            SymbolKind::Function(f) => Some(f),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_type_alias(&self) -> Option<&crate::type_alias::TypeAlias> {
        match self {
            SymbolKind::TypeAlias(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_type(&self) -> Option<&crate::types::Type> {
        match self {
            SymbolKind::Type(t) => Some(t),
            _ => None,
        }
    }

    pub fn as_type_mut(&mut self) -> Option<&mut crate::types::Type> {
        match self {
            SymbolKind::Type(t) => Some(t),
            _ => None,
        }
    }
}

/// The source of the symbol.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Source {
    pub file: PathBuf,
    pub offset_start_bytes: usize,
    pub offset_end_bytes: usize,
}
