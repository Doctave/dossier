use crate::{symbols::SymbolTable, ParserContext};
use dossier_core::{tree_sitter::Node, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TypeKind {
    Predefined(String),
    Identifier(String),
}

pub(crate) fn parse(
    node: &Node,
    _table: &mut SymbolTable,
    ctx: &ParserContext,
) -> Result<TypeKind> {
    match node.kind() {
        "predefined_type" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(TypeKind::Predefined(type_name))
        }
        "type_identifier" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(TypeKind::Identifier(type_name))
        }
        _ => panic!(
            "Unhandled type kind: {} | {} | {}",
            node.kind(),
            node.utf8_text(ctx.code.as_bytes()).unwrap(),
            node.to_sexp()
        ),
    }
}
