use crate::{
    symbol::{Source, Symbol, SymbolKind},
    ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "constraint";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeConstraint {
    pub identifier: String,
    pub extends: bool,
    pub key_of: bool,
    pub children: Vec<Symbol>,
}

impl TypeConstraint {
    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        let mut meta = json!({});
        if self.extends {
            meta["extends"] = true.into();
        }

        Entity {
            title: self.identifier.clone(),
            description: String::new(),
            kind: "type_constraint".to_owned(),
            identity: Identity::FQN(fqn.to_owned()),
            member_context: None,
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

    #[cfg(test)]
    pub fn the_type(&self) -> &Symbol {
        &self.children[0]
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut extends = false;
    let mut key_of = false;
    let mut cursor = node.walk();
    cursor.goto_first_child();

    if cursor.node().kind() == "extends" {
        extends = true;
        cursor.goto_next_sibling();
    }

    if cursor.node().kind() == "index_type_query" {
        key_of = true;
        cursor.goto_first_child();
        cursor.goto_next_sibling();
    }

    let the_type = crate::types::parse(&cursor.node(), ctx).unwrap();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::TypeConstraint(TypeConstraint {
            identifier: the_type.fqn.clone(),
            extends,
            key_of,
            children: vec![the_type],
        }),
        Source::for_node(node, ctx),
    ))
}
