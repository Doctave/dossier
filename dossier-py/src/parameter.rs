use dossier_core::{serde_json::json, tree_sitter::Node, Context, Entity, Result};

use crate::{
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Parameter {
    pub title: String,
    pub documentation: Option<String>,
    pub members: Vec<Symbol>,
}

impl Parameter {
    pub fn as_entity(&self, loc: &Location, context: Option<&SymbolContext>) -> Entity {
        unimplemented!()
    }
}

impl ParseSymbol for Parameter {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "typed_parameter" || node.kind() == "identifier"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &ParserContext) -> Result<Symbol> {
        assert!(
            node.kind() == "typed_parameter" || node.kind() == "identifier",
            "Expected typed_parameter or identifier"
        );

        // In this case, it's just a plain identifer
        if node.kind() == "identifier" {
            let title = node.utf8_text(ctx.code().as_bytes()).unwrap().to_owned();

            Ok(Symbol::new(
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

            let title = cursor.node()
                .utf8_text(ctx.code().as_bytes())
                .unwrap()
                .to_owned();

            let mut members = vec![];

            if let Some(type_node) = node.child_by_field_name("type") {
                // TODO
            }

            Ok(Symbol::new(
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