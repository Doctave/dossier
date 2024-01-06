use crate::{
    symbol::{Source, Symbol, SymbolKind},
    type_constraint, ParserContext,
};

use dossier_core::{tree_sitter::Node, Entity, Result};

pub(crate) const NODE_KIND: &str = "type_parameter";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeVariable {
    pub identifier: String,
    pub documentation: Option<String>,
    pub is_exported: bool,
    pub children: Vec<Symbol>,
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut children = vec![];
    let mut cursor = node.walk();

    cursor.goto_first_child();
    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    cursor.goto_next_sibling();

    loop {
        if cursor.node().kind() == type_constraint::NODE_KIND {
            children.push(type_constraint::parse(&cursor.node(), ctx)?);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::TypeVariable(TypeVariable {
            identifier,
            documentation: None,
            is_exported: false,
            children,
        }),
        Source::for_node(node, ctx),
    ))
}

impl TypeVariable {
    #[cfg(test)]
    pub fn constraints(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_type_constraint().is_some())
    }

    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }
}
