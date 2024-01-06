use crate::{
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};
use dossier_core::{tree_sitter::Node, Entity, Result};

pub(crate) const NODE_KIND: &str = "interface_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Interface {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Interfaces are actually just a single object type.
    /// We forward a bunch of methods to this child object.
    pub object_type: Box<Symbol>,
    pub exported: bool,
}

impl Interface {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }

    pub fn children(&self) -> &[Symbol] {
        self.object_type.children()
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        self.object_type.children_mut()
    }

    #[cfg(test)]
    pub fn properties(&self) -> impl Iterator<Item = &Symbol> {
        self.children().iter().filter(|s| s.kind.as_property().is_some())
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = node.walk();

    cursor.goto_first_child();
    cursor.goto_next_sibling();

    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    cursor.goto_next_sibling();

    debug_assert_eq!(cursor.node().kind(), "object_type");

    let object_type = types::parse(&cursor.node(), ctx).map(Box::new)?;

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Interface(Interface {
            identifier,
            documentation: None,
            object_type,
            exported: false,
        }),
        Source::for_node(node, ctx),
    ))
}
