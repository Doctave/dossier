use dossier_core::{serde_json::json, Entity, Result};

use crate::{
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    types::Type,
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter {
    pub title: String,
    pub documentation: Option<String>,
    pub members: Vec<Symbol>,
}

impl Parameter {
    pub fn as_entity(
        &self,
        loc: &Location,
        fqn: Option<&str>,
        context: Option<&SymbolContext>,
    ) -> Entity {
        Entity {
            title: Some(self.title.to_owned()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "parameter".to_owned(),
            identity: dossier_core::Identity::FQN(
                fqn.expect("parameter without FQN").to_owned(),
            ),
            members: self.members.iter().map(|s| s.as_entity()).collect(),
            member_context: context.map(|_| "method".to_owned()),
            language: crate::LANGUAGE.to_owned(),
            source: loc.as_source(),
            meta: json!({}),
        }
    }

    #[cfg(test)]
    pub fn the_type(&self) -> Option<&Symbol> {
        self.members.iter().find(|s| s.as_type().is_some())
    }
}

impl ParseSymbol for Parameter {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "typed_parameter"
            || node.kind() == "identifier"
            || node.kind() == "typed_default_parameter"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &mut ParserContext) -> Result<Symbol> {
        assert!(
            <Parameter as ParseSymbol>::matches_node(node),
            "Not valid type"
        );

        // In this case, it's just a plain identifer
        if node.kind() == "identifier" {
            let title = node.utf8_text(ctx.code().as_bytes()).unwrap().to_owned();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Parameter(Parameter {
                    title,
                    documentation: None,
                    members: vec![],
                }),
                Location::new(&node, ctx),
            ))
        } else {
            // In this case, it's a typed parameter:
            // (typed_parameter (identifier) type: (type (identifier)))
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let title = cursor
                .node()
                .utf8_text(ctx.code().as_bytes())
                .unwrap()
                .to_owned();

            let mut members = vec![];

            if let Some(type_node) = node.child_by_field_name("type") {
                // TODO
                if Type::matches_node(type_node) {
                    members.push(Type::parse_symbol(type_node, ctx)?);
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Parameter(Parameter {
                    title,
                    documentation: None,
                    members,
                }),
                Location::new(&node, ctx),
            ))
        }
    }
}
