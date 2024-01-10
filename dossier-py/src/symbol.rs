use crate::ParserContext;

use dossier_core::{Entity, Result, Source};
use tree_sitter::Node;

use std::path::PathBuf;

pub(crate) trait ParseSymbol {
    fn matches_node(node: Node) -> bool;
    fn parse_symbol(node: Node, ctx: &ParserContext) -> Result<Symbol>;
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Location {
    file: PathBuf,
    start_offset_bytes: usize,
    end_offset_bytes: usize,
}

impl Location {
    pub fn new(node: &Node, ctx: &ParserContext) -> Self {
        Location {
            file: ctx.file().to_path_buf(),
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
pub(crate) struct Symbol {
    pub kind: SymbolKind,
    pub loc: Location,
    pub context: Option<SymbolContext>,
}

impl Symbol {
    pub fn new(kind: SymbolKind, loc: Location) -> Self {
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
            SymbolKind::Parameter(s) => s.as_entity(&self.loc, self.context.as_ref()),
        }
    }

    #[cfg(test)]
    pub fn as_class(&self) -> Option<&crate::class::Class> {
        match &self.kind {
            SymbolKind::Class(class) => Some(class),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_function(&self) -> Option<&crate::function::Function> {
        match &self.kind {
            SymbolKind::Function(f) => Some(f),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn as_parameter(&self) -> Option<&crate::parameter::Parameter> {
        match &self.kind {
            SymbolKind::Parameter(parameter) => Some(parameter),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolKind {
    Class(crate::class::Class),
    Function(crate::function::Function),
    Parameter(crate::parameter::Parameter),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SymbolContext {
    Method,
}
