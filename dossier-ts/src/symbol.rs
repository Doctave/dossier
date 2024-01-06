use std::path::PathBuf;

use dossier_core::Entity;
use tree_sitter::Node;

use crate::{symbol_table::ScopeID, ParserContext};

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
    pub scope_id: ScopeID,
}

impl Symbol {
    pub fn in_context(ctx: &ParserContext, kind: SymbolKind, source: Source) -> Self {
        let fqn = ctx.construct_fqn(kind.identifier());
        let scope_id = ctx.current_scope();
        let context = ctx.symbol_context().cloned();

        Self {
            kind,
            source,
            fqn,
            context,
            scope_id,
        }
    }

    pub fn is_exported(&self) -> bool {
        match &self.kind {
            SymbolKind::TypeAlias(a) => a.exported,
            _ => false,
        }
    }

    pub fn mark_as_exported(&mut self) {
        match &mut self.kind {
            SymbolKind::TypeAlias(ref mut a) => a.exported = true,
            _ => {}
        }
    }

    pub fn as_entity(&self) -> Entity {
        match &self.kind {
            SymbolKind::Function(f) => f.as_entity(&self.source, &self.fqn),
            SymbolKind::TypeAlias(a) => a.as_entity(&self.source, &self.fqn),
            SymbolKind::Type(t) => t.as_entity(&self.source, &self.fqn),
            SymbolKind::Property(p) => p.as_entity(&self.source, &self.fqn),
            SymbolKind::TypeVariable(t) => t.as_entity(&self.source, &self.fqn),
        }
    }

    pub fn identifier(&self) -> &str {
        self.kind.identifier()
    }

    pub fn children(&self) -> &[Symbol] {
        match &self.kind {
            SymbolKind::Function(f) => f.children.as_slice(),
            SymbolKind::TypeAlias(a) => a.children.as_slice(),
            SymbolKind::Type(t) => t.children(),
            SymbolKind::Property(p) => p.children.as_slice(),
            SymbolKind::TypeVariable(t) => t.children.as_slice(),
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self.kind {
            SymbolKind::Function(ref mut f) => f.children.as_mut_slice(),
            SymbolKind::TypeAlias(ref mut a) => a.children.as_mut_slice(),
            SymbolKind::Type(ref mut t) => t.children_mut(),
            SymbolKind::Property(ref mut p) => p.children.as_mut_slice(),
            SymbolKind::TypeVariable(ref mut t) => t.children.as_mut_slice(),
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
    Constraint,
}

/// The type of the symbol.
/// Contains all the metadata associated with that type of symbol
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Function(crate::function::Function),
    TypeAlias(crate::type_alias::TypeAlias),
    Type(crate::types::Type),
    TypeVariable(crate::type_variable::TypeVariable),
    Property(crate::property::Property),
}

impl SymbolKind {
    pub fn identifier(&self) -> &str {
        match &self {
            SymbolKind::Function(f) => f.identifier.as_str(),
            SymbolKind::TypeAlias(a) => a.identifier.as_str(),
            SymbolKind::Type(t) => t.identifier(),
            SymbolKind::Property(p) => p.identifier.as_str(),
            SymbolKind::TypeVariable(t) => t.identifier.as_str(),
        }
    }

    #[cfg(test)]
    pub fn as_type_variable(&self) -> Option<&crate::type_variable::TypeVariable> {
        match self {
            SymbolKind::TypeVariable(f) => Some(f),
            _ => None,
        }
    }

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

impl Source {
    pub fn for_node(node: &Node, ctx: &ParserContext) -> Self {
        let offset_start_bytes = node.start_byte();
        let offset_end_bytes = node.end_byte();

        Self {
            file: ctx.file.to_owned(),
            offset_start_bytes,
            offset_end_bytes,
        }
    }
}
