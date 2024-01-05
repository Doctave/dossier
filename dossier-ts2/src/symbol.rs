use std::path::PathBuf;

use dossier_core::Entity;

#[derive(Debug, Clone, PartialEq)]
/// A symbol we've discovered in the source code.
pub(crate) struct Symbol {
    pub kind: SymbolKind,
    pub source: Source,
    pub fqn: String,
    /// If this symbol is a child of another symbol, this
    /// field describes the relationship to its parent.
    ///
    /// E.g. a function would have a return type symbol,
    /// where this field would be set to SymbolContext::ReturnType
    pub context: Option<SymbolContext>,
}

impl Symbol {
    pub fn is_exported(&self) -> bool {
        match &self.kind {
            SymbolKind::TypeAlias(a) => a.exported,
            _ => false,
        }
    }

    pub fn mark_as_exported(&mut self) {
        match &mut self.kind {
            SymbolKind::TypeAlias(ref mut a) => a.exported = true,
            _ => {},
        }
    }

    pub fn as_entity(&self) -> Entity {
        match &self.kind {
            SymbolKind::Function(f) => f.as_entity(&self.source, &self.fqn),
            SymbolKind::TypeAlias(a) => a.as_entity(&self.source, &self.fqn),
            SymbolKind::Type(t) => t.as_entity(&self.source, &self.fqn),
            SymbolKind::Property(p) => p.as_entity(&self.source, &self.fqn),
        }
    }

    pub fn identifier(&self) -> &str {
        match &self.kind {
            SymbolKind::Function(f) => f.identifier.as_str(),
            SymbolKind::TypeAlias(a) => a.identifier.as_str(),
            SymbolKind::Type(t) => t.identifier(),
            SymbolKind::Property(p) => p.identifier.as_str(),
        }
    }

    pub fn children(&self) -> &[Symbol] {
        match &self.kind {
            SymbolKind::Function(f) => f.children.as_slice(),
            SymbolKind::TypeAlias(a) => a.children.as_slice(),
            SymbolKind::Type(t) => t.children(),
            SymbolKind::Property(p) => p.children.as_slice(),
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self.kind {
            SymbolKind::Function(ref mut f) => f.children.as_mut_slice(),
            SymbolKind::TypeAlias(ref mut a) => a.children.as_mut_slice(),
            SymbolKind::Type(ref mut t) => t.children_mut(),
            SymbolKind::Property(ref mut p) => p.children.as_mut_slice(),
        }
    }

    pub fn resolvable_identifier(&self) -> Option<&str> {
        match &self.kind {
            SymbolKind::Type(t) => t.resolvable_identifier(),
            _ => None,
        }
    }

    pub fn resolve_type(&mut self, fqn: &str) {
        if let SymbolKind::Type(t) = &mut self.kind {
            t.resolve_type(fqn)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolContext {
    ReturnType,
    Property,
}

/// The type of the symbol.
/// Contains all the metadata associated with that type of symbol
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Function(crate::function::Function),
    TypeAlias(crate::type_alias::TypeAlias),
    Type(crate::types::Type),
    Property(crate::property::Property),
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

    #[cfg(test)]
    pub fn as_type(&self) -> Option<&crate::types::Type> {
        match self {
            SymbolKind::Type(t) => Some(t),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_property(&self) -> Option<&crate::property::Property> {
        match self {
            SymbolKind::Property(p) => Some(p),
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
