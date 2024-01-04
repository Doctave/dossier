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
    pub fn identifier(&self) -> &str {
        match self {
            Type::Predefined(type_name) => type_name.as_str(),
            Type::Identifier(identifier, _) => identifier.as_str(),
        }
    }

    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    match node.kind() {
        "predefined_type" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol {
                fqn: ctx.construct_fqn(&type_name),
                kind: SymbolKind::Type(Type::Predefined(type_name)),
                source: Source {
                    file: ctx.file.to_owned(),
                    offset_start_bytes: node.start_byte(),
                    offset_end_bytes: node.end_byte(),
                },
            })
        }
        "type_identifier" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol {
                fqn: ctx.construct_fqn(&type_name),
                kind: SymbolKind::Type(Type::Identifier(type_name, None)),
                source: Source {
                    file: ctx.file.to_owned(),
                    offset_start_bytes: node.start_byte(),
                    offset_end_bytes: node.end_byte(),
                },
            })
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
mod test {
    // use super::*;
    // use dossier_core::tree_sitter::Parser;
    // use indoc::indoc;
    // use std::path::Path;

    // #[test]
    // fn parses_predefined_type() {
    //     let code = indoc! {r#"
    //         type Foo = string;
    //     #"#};

    //     let mut ctx = ParserContext::new(Path::new("index.ts"), code);

    //     let mut parser = Parser::new();

    //     parser
    //         .set_language(tree_sitter_typescript::language_typescript())
    //         .expect("Error loading TypeScript grammar");

    //     let tree = parser.parse(ctx.code, None).unwrap();
    //     // Walk to the correct type node
    //     let mut cursor = tree.walk();
    //     cursor.goto_first_child();
    //     cursor.goto_first_child();
    //     cursor.goto_next_sibling();
    //     cursor.goto_next_sibling();
    //     cursor.goto_next_sibling();

    //     println!("{:?} | {}", cursor.node().kind(), cursor.node().to_sexp());
    // }
}
