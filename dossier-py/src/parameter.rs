use dossier_core::{Entity, Result};

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
    pub fn as_entity(&self, loc: &Location, context: Option<&SymbolContext>) -> Entity {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn the_type(&self) -> Option<&Symbol> {
        self.members.iter().find(|s| s.as_type().is_some())
    }
}

impl ParseSymbol for Parameter {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "typed_parameter" || node.kind() == "identifier"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &mut ParserContext) -> Result<Symbol> {
        assert!(
            node.kind() == "typed_parameter" || node.kind() == "identifier",
            "Expected typed_parameter or identifier"
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
