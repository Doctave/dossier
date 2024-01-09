use dossier_core::{tree_sitter::Node, Result};

use crate::{
    symbol::{Location, ParseSymbol, Symbol, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Class<'a> {
    pub title: &'a str,
    pub documentation: Option<String>,
}

impl<'a> ParseSymbol<'a> for Class<'a> {
    fn matches_node(node: tree_sitter::Node<'a>) -> bool {
        node.kind() == "class_definition"
    }

    fn parse_symbol(node: tree_sitter::Node<'a>, ctx: &'a ParserContext) -> Result<Symbol<'a>> {
        assert_eq!(node.kind(), "class_definition", "Expected class definition");

        let title = node
            .child_by_field_name("name")
            .expect("Expected class name")
            .utf8_text(ctx.code().as_bytes())
            .unwrap();

        let documentation = find_docs(&node, ctx);

        Ok(Symbol::new(
            SymbolKind::Class(Class {
                title,
                documentation,
            }),
            Location::new(&node, ctx),
        ))
    }
}

fn find_docs<'a>(node: &Node<'a>, ctx: &'a ParserContext) -> Option<String> {
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
