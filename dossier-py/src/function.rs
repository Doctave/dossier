use dossier_core::{serde_json::json, tree_sitter::Node, Entity, Result};

use crate::{
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function {
    pub title: String,
    pub documentation: Option<String>,
}

impl Function {
    pub fn as_entity(&self, loc: &Location, context: Option<&SymbolContext>) -> Entity {
        Entity {
            title: Some(self.title.to_owned()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "function".to_owned(),
            identity: dossier_core::Identity::FQN("TODO".to_owned()),
            members: vec![],
            member_context: context.map(|_| "method".to_owned()),
            language: crate::LANGUAGE.to_owned(),
            source: loc.as_source(),
            meta: json!({}),
        }
    }
}

impl ParseSymbol for Function {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "function_definition"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &ParserContext) -> Result<Symbol> {
        assert_eq!(
            node.kind(),
            "function_definition",
            "Expected function definition"
        );

        let title = node
            .child_by_field_name("name")
            .expect("Expected class name")
            .utf8_text(ctx.code().as_bytes())
            .unwrap()
            .to_owned();

        let documentation = find_docs(&node, ctx);

        Ok(Symbol::new(
            SymbolKind::Function(Function {
                title,
                documentation,
            }),
            Location::new(&node, ctx),
        ))
    }
}

fn find_docs(node: &Node, ctx: &ParserContext) -> Option<String> {
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        cursor.goto_first_child();

        if cursor.node().kind() == "expression_statement" {
            cursor.goto_first_child();
            if cursor.node().kind() == "string" {
                let possible_docs = cursor.node().utf8_text(ctx.code().as_bytes()).unwrap();
                crate::helpers::process_docs(possible_docs)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}
