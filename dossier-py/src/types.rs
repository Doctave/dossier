use dossier_core::{serde_json::json, tree_sitter::Node, Context, Entity, Result};

use crate::{
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Type {
    BuiltIn(String),
    Identifier(String),
}

impl Type {
    pub fn as_entity(&self, loc: &Location, context: Option<&SymbolContext>) -> Entity {
        unimplemented!()
    }
}

impl ParseSymbol for Type {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "type"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &ParserContext) -> Result<Symbol> {
        assert_eq!(node.kind(), "type", "Expected type");

        let mut cursor = node.walk();
        cursor.goto_first_child();

        let title = cursor
            .node()
            .utf8_text(ctx.code().as_bytes())
            .unwrap()
            .to_owned();

        let out = if is_built_in(&title) {
            Type::BuiltIn(title)
        } else {
            Type::Identifier(title)
        };

        Ok(Symbol::new(
            SymbolKind::Type(out),
            Location::new(&node, ctx),
        ))
    }
}

fn is_built_in(title: &str) -> bool {
    matches!(
        title,
        "int" | "string" | "bool" | "float" | "double" | "void"
    )
}
