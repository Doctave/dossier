use crate::{
    symbols::{Source, Symbol, SymbolKind},
    ParserContext,
};
use dossier_core::{tree_sitter::Node, Entity, Result};

type ResolvedTypeFQN = String;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Type {
    Predefined(String),
    /// This is the case where we have a type alias, and we need to resolve it.
    ///
    /// When the type has been resolved, the second element in the tuple will
    /// contain the FQN of the type.
    Identifier(String, Option<ResolvedTypeFQN>),
}

impl Type {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<(String, Symbol)> {
    match node.kind() {
        "predefined_type" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok((
                "todo".to_string(),
                Symbol {
                    fqn: ctx.construct_fqn(&type_name),
                    kind: SymbolKind::Type(Type::Predefined(type_name)),
                    source: Source {
                        file: ctx.file.to_owned(),
                        offset_start_bytes: node.start_byte(),
                        offset_end_bytes: node.end_byte(),
                    },
                },
            ))
        }
        "type_identifier" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok((
                "todo".to_string(),
                Symbol {
                    fqn: ctx.construct_fqn(&type_name),
                    kind: SymbolKind::Type(Type::Identifier(type_name, None)),
                    source: Source {
                        file: ctx.file.to_owned(),
                        offset_start_bytes: node.start_byte(),
                        offset_end_bytes: node.end_byte(),
                    },
                },
            ))
        }
        _ => panic!(
            "Unhandled type kind: {} | {} | {}",
            node.kind(),
            node.utf8_text(ctx.code.as_bytes()).unwrap(),
            node.to_sexp()
        ),
    }
}

#[cfg(test)]
mod test {}
