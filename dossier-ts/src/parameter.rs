use crate::{
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    types, ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KINDS: &[&str] = &["required_parameter", "optional_parameter"];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter {
    pub identifier: String,
    /// Technically will ever only have one child, the type itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub optional: bool,
    pub readonly: bool,
}

impl Parameter {
    pub fn as_entity(
        &self,
        source: &Source,
        fqn: Option<&str>,
        symbol_context: Option<SymbolContext>,
    ) -> Entity {
        let mut meta = json!({});
        if self.optional {
            meta["optional"] = true.into();
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: String::new(),
            kind: "parameter".to_owned(),
            identity: Identity::FQN(fqn.expect("Parameter without FQN").to_owned()),
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
    pub fn parameter_type(&self) -> Option<&Symbol> {
        self.children.first()
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert!(NODE_KINDS.contains(&node.kind()));

    let mut children = vec![];
    let mut cursor = node.walk();
    cursor.goto_first_child();

    let mut optional = false;
    let mut readonly = false;

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

        if cursor.node().kind() == "readonly_type" {
            readonly = true;
            cursor.goto_first_child();
            cursor.goto_next_sibling();
        }
        children.push(types::parse(&cursor.node(), ctx)?);
    }

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Parameter(Parameter {
            identifier,
            children,
            optional,
            readonly,
        }),
        Source::for_node(node, ctx),
    ))
}
