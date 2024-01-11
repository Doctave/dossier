use crate::{
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    type_constraint, ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "type_parameter";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeVariable {
    pub identifier: String,
    pub documentation: Option<String>,
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

    pub fn as_entity(
        &self,
        source: &Source,
        fqn: Option<&str>,
        symbol_context: Option<SymbolContext>,
    ) -> Entity {
        let meta = json!({});

        Entity {
            title: Some(self.identifier.clone()),
            description: String::new(),
            kind: "type_constraint".to_owned(),
            identity: Identity::FQN(fqn.expect("Generic type variable withou FQN").to_owned()),
            member_context: symbol_context.map(|sc| sc.to_string()),
            language: crate::LANGUAGE.to_owned(),
            source: source.as_entity_source(),
            meta,
            members: self
                .children
                .iter()
                .map(|s| s.as_entity())
                .collect::<Vec<_>>(),
        }
    }
}
