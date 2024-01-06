use crate::{
    symbol::{Source, Symbol, SymbolKind},
    ParserContext,
};

use dossier_core::{tree_sitter::Node, Entity, Result};

pub(crate) const NODE_KIND: &str = "constraint";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeConstraint {
    pub identifier: String,
    pub extends: bool,
    pub children: Vec<Symbol>,
}

impl TypeConstraint {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn the_type(&self) -> &Symbol {
        &self.children[0]
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut extends = false;
    let mut cursor = node.walk();
    cursor.goto_first_child();

    if cursor.node().kind() == "extends" {
        extends = true;
        cursor.goto_next_sibling();
    }

    let the_type = crate::types::parse(&cursor.node(), ctx).unwrap();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::TypeConstraint(TypeConstraint {
            identifier: the_type.fqn.clone(),
            extends,
            children: vec![the_type],
        }),
        Source::for_node(node, ctx),
    ))
}
