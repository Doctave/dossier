use crate::ParserContext;

use dossier_core::{Entity, Result, Source};
use tree_sitter::Node;

use std::path::Path;

pub(crate) trait ParseSymbol<'a> {
    fn matches_node(node: Node<'a>) -> bool;
    fn parse_symbol(node: Node<'a>, ctx: &'a ParserContext) -> Result<Symbol<'a>>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Location<'a> {
    file: &'a Path,
    start_offset_bytes: usize,
    end_offset_bytes: usize,
}

impl<'a> Location<'a> {
    pub fn new(node: &Node<'a>, ctx: &'a ParserContext) -> Self {
        Location {
            file: ctx.file(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
        }
    }

    pub fn as_source(&self) -> Source {
        Source {
            file: self.file.to_path_buf(),
            start_offset_bytes: self.start_offset_bytes,
            end_offset_bytes: self.end_offset_bytes,
            repository: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Symbol<'a> {
    pub kind: SymbolKind<'a>,
    pub loc: Location<'a>,
    pub context: Option<SymbolContext>,
}

impl<'a> Symbol<'a> {
    pub fn new(kind: SymbolKind<'a>, loc: Location<'a>) -> Self {
        Symbol {
            kind,
            loc,
            context: None,
        }
    }

    pub fn as_entity(&self) -> Entity {
        match &self.kind {
            SymbolKind::Class(s) => s.as_entity(&self.loc, self.context.as_ref()),
            SymbolKind::Function(s) => s.as_entity(&self.loc, self.context.as_ref()),
        }
    }

    #[cfg(test)]
    pub fn as_class(&self) -> Option<&crate::class::Class<'a>> {
        match &self.kind {
            SymbolKind::Class(class) => Some(class),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_function(&self) -> Option<&crate::function::Function<'a>> {
        match &self.kind {
            SymbolKind::Function(f) => Some(f),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind<'a> {
    Class(crate::class::Class<'a>),
    Function(crate::function::Function<'a>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolContext {
    Method,
}
