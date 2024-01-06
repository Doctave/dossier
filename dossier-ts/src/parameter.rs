use crate::{
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};
use dossier_core::{tree_sitter::Node, Entity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter {
    pub identifier: String,
    /// Technically will ever only have one child, the type itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub optional: bool,
}

impl Parameter {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn parameter_type(&self) -> Option<&Symbol> {
        self.children.get(0)
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert!(matches!(
        node.kind(),
        "required_parameter" | "optional_parameter"
    ));

    let mut children = vec![];
    let mut cursor = node.walk();
    cursor.goto_first_child();

    let mut optional = false;
    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    if cursor.goto_next_sibling() && cursor.node().kind() == "?" {
        optional = true;
        cursor.goto_next_sibling();
    }

    if cursor.node().kind() == "type_annotation" {
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        children.push(types::parse(&cursor.node(), ctx)?);
    }

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Parameter(Parameter {
            identifier,
            children,
            optional,
        }),
        Source::for_node(node, ctx),
    ))
}
