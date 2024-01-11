use std::path::PathBuf;

use dossier_core::Entity;
use std::sync::atomic::AtomicUsize;
use tree_sitter::Node;

use crate::{symbol_table::ScopeID, ParserContext};

static SYMBOL_ID: AtomicUsize = AtomicUsize::new(1);

pub(crate) const UNUSED_SYMBOL_ID: usize = 0;
pub(crate) type SymbolID = usize;

#[derive(Debug, Clone, PartialEq)]
/// A symbol we've discovered in the source code.
pub(crate) struct Symbol {
    pub id: usize,
    pub kind: SymbolKind,
    pub source: Source,
    pub fqn: Option<String>,
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
        let fqn = kind.identifier().map(|i| ctx.construct_fqn(i));
        let scope_id = ctx.current_scope();
        let context = ctx.symbol_context().cloned();

        Self {
            id: SYMBOL_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
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
        #[allow(clippy::single_match)]
        match &mut self.kind {
            SymbolKind::TypeAlias(ref mut a) => a.exported = true,
            _ => {}
        }
    }

    pub fn as_entity(&self) -> Entity {
        match &self.kind {
            SymbolKind::Class(c) => c.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Function(f) => f.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Field(f) => f.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Interface(i) => i.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Method(m) => m.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::TypeAlias(a) => a.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Type(t) => t.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Parameter(p) => p.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::Property(p) => p.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::TypeVariable(t) => t.as_entity(&self.source, self.fqn.as_deref()),
            SymbolKind::TypeConstraint(t) => t.as_entity(&self.source, self.fqn.as_deref()),
        }
    }

    #[cfg(test)]
    pub fn identifier(&self) -> Option<&str> {
        self.kind.identifier()
    }

    pub fn children(&self) -> &[Symbol] {
        match &self.kind {
            SymbolKind::Class(c) => c.children.as_slice(),
            SymbolKind::Function(f) => f.children.as_slice(),
            SymbolKind::Field(f) => f.children.as_slice(),
            SymbolKind::Interface(i) => i.children.as_slice(),
            SymbolKind::Method(m) => m.children.as_slice(),
            SymbolKind::TypeAlias(a) => a.children.as_slice(),
            SymbolKind::Type(t) => t.children(),
            SymbolKind::Parameter(p) => p.children.as_slice(),
            SymbolKind::Property(p) => p.children.as_slice(),
            SymbolKind::TypeVariable(t) => t.children.as_slice(),
            SymbolKind::TypeConstraint(t) => t.children.as_slice(),
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self.kind {
            SymbolKind::Class(ref mut c) => c.children.as_mut_slice(),
            SymbolKind::Function(ref mut f) => f.children.as_mut_slice(),
            SymbolKind::Field(ref mut f) => f.children.as_mut_slice(),
            SymbolKind::Interface(ref mut i) => i.children.as_mut_slice(),
            SymbolKind::Method(ref mut m) => m.children.as_mut_slice(),
            SymbolKind::TypeAlias(ref mut a) => a.children.as_mut_slice(),
            SymbolKind::Type(ref mut t) => t.children_mut(),
            SymbolKind::Parameter(ref mut p) => p.children.as_mut_slice(),
            SymbolKind::Property(ref mut p) => p.children.as_mut_slice(),
            SymbolKind::TypeVariable(ref mut t) => t.children.as_mut_slice(),
            SymbolKind::TypeConstraint(ref mut t) => t.children.as_mut_slice(),
        }
    }

    pub fn resolvable_identifier(&self) -> Option<&str> {
        match &self.kind {
            SymbolKind::Type(t) => t.resolvable_identifier(),
            SymbolKind::TypeAlias(t) => Some(t.identifier.as_str()),
            SymbolKind::Interface(i) => Some(i.identifier.as_str()),
            SymbolKind::Class(i) => Some(i.identifier.as_str()),
            SymbolKind::TypeVariable(t) => Some(t.identifier.as_str()),
            SymbolKind::Function(f) => Some(f.identifier.as_str()),
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
    Extends,
}

/// The type of the symbol.
/// Contains all the metadata associated with that type of symbol
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Class(crate::class::Class),
    Field(crate::field::Field),
    Function(crate::function::Function),
    Interface(crate::interface::Interface),
    Method(crate::method::Method),
    TypeAlias(crate::type_alias::TypeAlias),
    Type(crate::types::Type),
    TypeVariable(crate::type_variable::TypeVariable),
    TypeConstraint(crate::type_constraint::TypeConstraint),
    Parameter(crate::parameter::Parameter),
    Property(crate::property::Property),
}

impl SymbolKind {
    pub fn identifier(&self) -> Option<&str> {
        match &self {
            SymbolKind::Class(c) => Some(c.identifier.as_str()),
            SymbolKind::Function(f) => Some(f.identifier.as_str()),
            SymbolKind::Field(f) => Some(f.identifier.as_str()),
            SymbolKind::Interface(i) => Some(i.identifier.as_str()),
            SymbolKind::Method(m) => Some(m.identifier.as_str()),
            SymbolKind::TypeAlias(a) => Some(a.identifier.as_str()),
            SymbolKind::Type(t) => t.identifier(),
            SymbolKind::Parameter(p) => Some(p.identifier.as_str()),
            SymbolKind::Property(p) => Some(p.identifier.as_str()),
            SymbolKind::TypeVariable(t) => Some(t.identifier.as_str()),
            SymbolKind::TypeConstraint(_) => None,
        }
    }

    #[cfg(test)]
    pub fn as_class(&self) -> Option<&crate::class::Class> {
        match self {
            SymbolKind::Class(c) => Some(c),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_field(&self) -> Option<&crate::field::Field> {
        match self {
            SymbolKind::Field(f) => Some(f),
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
    pub fn as_interface(&self) -> Option<&crate::interface::Interface> {
        match self {
            SymbolKind::Interface(f) => Some(f),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_method(&self) -> Option<&crate::method::Method> {
        match self {
            SymbolKind::Method(m) => Some(m),
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
    pub fn as_type_variable(&self) -> Option<&crate::type_variable::TypeVariable> {
        match self {
            SymbolKind::TypeVariable(f) => Some(f),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_type_constraint(&self) -> Option<&crate::type_constraint::TypeConstraint> {
        match self {
            SymbolKind::TypeConstraint(t) => Some(t),
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

    #[cfg(test)]
    pub fn as_parameter(&self) -> Option<&crate::parameter::Parameter> {
        match self {
            SymbolKind::Parameter(p) => Some(p),
            _ => None,
        }
    }
}

pub(crate) struct SymbolIterator<'a> {
    stack: std::collections::VecDeque<&'a Symbol>,
}

impl<'a> SymbolIterator<'a> {
    pub fn new(symbols: &'a [Symbol]) -> Self {
        let stack = symbols.iter().collect();
        SymbolIterator { stack }
    }
}

impl<'a> Iterator for SymbolIterator<'a> {
    type Item = &'a Symbol;

    fn next(&mut self) -> Option<Self::Item> {
        self.stack.pop_front().map(|symbol| {
            self.stack.extend(symbol.children());
            symbol
        })
    }
}

/// The source of the symbol.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Source {
    pub file: PathBuf,
    pub start_offset_bytes: usize,
    pub end_offset_bytes: usize,
}

impl Source {
    pub fn for_node(node: &Node, ctx: &ParserContext) -> Self {
        let offset_start_bytes = node.start_byte();
        let offset_end_bytes = node.end_byte();

        Self {
            file: ctx.file.to_owned(),
            start_offset_bytes: offset_start_bytes,
            end_offset_bytes: offset_end_bytes,
        }
    }

    pub fn as_entity_source(&self) -> dossier_core::Source {
        dossier_core::Source {
            file: self.file.to_owned(),
            start_offset_bytes: self.start_offset_bytes,
            end_offset_bytes: self.end_offset_bytes,
            repository: None,
        }
    }
}
