use dossier_core::{serde_json::json, Entity, Result};

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
    pub fn as_entity(
        &self,
        loc: &Location,
        fqn: Option<&str>,
        context: Option<&SymbolContext>,
    ) -> Entity {
        Entity {
            title: self.identifier().map(|i| i.to_owned()),
            description: String::new(),
            kind: "type".to_owned(),
            identity: match fqn {
                Some(f) => dossier_core::Identity::FQN(f.to_owned()),
                None => dossier_core::Identity::Anonymous
            },
            members: vec![],
            member_context: context.map(|c| c.to_string()),
            language: crate::LANGUAGE.to_owned(),
            source: loc.as_source(),
            meta: json!({}),
        }
    }

    pub fn identifier(&self) -> Option<&str> {
        match self {
            Type::BuiltIn(s) => Some(s),
            Type::Identifier(s) => Some(s),
        }
    }
}

impl ParseSymbol for Type {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "type"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &mut ParserContext) -> Result<Symbol> {
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

        Ok(Symbol::in_context(
            ctx,
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
