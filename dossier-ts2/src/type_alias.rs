use crate::type_kind::TypeKind;
use crate::{
    symbols::{Source, Symbol, SymbolKind},
    type_kind, ParserContext,
};
use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeAlias {
    pub identifier: String,
    pub type_kind: TypeKind,
}

impl TypeAlias {
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
}

pub(crate) const NODE_KIND: &str = "type_alias_declaration";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<(String, Symbol)> {
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

    while !cursor.node().is_named() {
        cursor.goto_next_sibling();
    }

    let type_kind = type_kind::parse(&cursor.node(), ctx)?;

    Ok((
        identifier.clone(),
        Symbol {
            fqn: ctx.construct_fqn(&identifier),
            kind: SymbolKind::TypeAlias(TypeAlias {
                identifier,
                type_kind,
            }),
            source: Source {
                file: ctx.file.to_owned(),
                offset_start_bytes: node.start_byte(),
                offset_end_bytes: node.end_byte(),
            },
        },
    ))
}
