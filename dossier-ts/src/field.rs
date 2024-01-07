use crate::{
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};
use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Field {
    pub identifier: String,
    /// Technically will ever only have max one child, the value itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub readonly: bool,
    pub private: bool,
}

impl Field {
    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        Entity {
            title: self.identifier.clone(),
            description: String::new(),
            kind: "type_alias".to_owned(),
            identity: dossier_core::Identity::FQN(fqn.to_owned()),
            members: vec![],
            member_context: None,
            language: crate::LANGUAGE.to_owned(),
            source: dossier_core::Source {
                file: source.file.to_owned(),
                start_offset_bytes: source.offset_start_bytes,
                end_offset_bytes: source.offset_end_bytes,
                repository: None,
            },
            meta: json!({}),
        }
    }

    #[cfg(test)]
    pub fn value(&self) -> Option<&Symbol> {
        self.children.get(0)
    }
}

pub(crate) const NODE_KIND: &str = "public_field_definition";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = node.walk();
    cursor.goto_first_child();

    while !cursor.node().is_named() {
        cursor.goto_next_sibling();
    }

    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    cursor.goto_next_sibling();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Field(Field {
            identifier,
            children: vec![],
            readonly: false,
            private: false,
        }),
        Source {
            file: ctx.file.to_owned(),
            offset_start_bytes: node.start_byte(),
            offset_end_bytes: node.end_byte(),
        },
    ))
}
