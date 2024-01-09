use crate::ParserContext;

use dossier_core::{Entity, Result};
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
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Symbol<'a> {
    kind: SymbolKind<'a>,
    loc: Location<'a>,
}

impl<'a> Symbol<'a> {
    pub fn new(kind: SymbolKind<'a>, loc: Location<'a>) -> Self {
        Symbol { kind, loc }
    }

    pub fn as_entity(&self) -> Entity {
        unimplemented!()
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
