use crate::type_kind::TypeKind;
use crate::{
    symbols::{Source, Symbol, SymbolKind, SymbolTable},
    type_kind, ParserContext,
};
use dossier_core::{tree_sitter::Node, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeAlias {
    pub identifier: String,
    pub type_kind: TypeKind,
}

pub(crate) const NODE_KIND: &str = "type_alias_declaration";

pub(crate) fn parse(
    node: &Node,
    table: &mut SymbolTable,
    ctx: &ParserContext,
) -> Result<(String, Symbol)> {
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

    let type_kind = type_kind::parse(&cursor.node(), table, ctx)?;

    Ok((
        identifier.clone(),
        Symbol {
            kind: SymbolKind::TypeAlias(TypeAlias {
                identifier,
                type_kind,
            }),
            source: Source {
                offset_start_bytes: node.start_byte(),
                offset_end_bytes: node.end_byte(),
            },
        },
    ))
}
