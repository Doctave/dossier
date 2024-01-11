use crate::{
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "constraint";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeConstraint {
    pub extends: bool,
    pub children: Vec<Symbol>,
}

impl TypeConstraint {
    pub fn as_entity(
        &self,
        source: &Source,
        _fqn: Option<&str>,
        symbol_context: Option<SymbolContext>,
    ) -> Entity {
        let mut meta = json!({});
        if self.extends {
            meta["extends"] = true.into();
        }

        Entity {
            title: None,
            description: String::new(),
            kind: "type_constraint".to_owned(),
            identity: Identity::Anonymous,
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
            extends,
            children: vec![the_type],
        }),
        Source::for_node(node, ctx),
    ))
}
