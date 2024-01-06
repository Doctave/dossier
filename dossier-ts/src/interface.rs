use crate::{
    helpers::*,
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};
use dossier_core::{tree_sitter::Node, Entity, Result};

pub(crate) const NODE_KIND: &str = "interface_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Interface {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Interfaces are actually just a single object type.
    /// We forward a bunch of methods to this child object.
    pub object_type: Box<Symbol>,
    pub exported: bool,
}

impl Interface {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }

    pub fn children(&self) -> &[Symbol] {
        self.object_type.children()
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        self.object_type.children_mut()
    }

    #[cfg(test)]
    pub fn properties(&self) -> impl Iterator<Item = &Symbol> {
        self.children()
            .iter()
            .filter(|s| s.kind.as_property().is_some())
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = node.walk();

    cursor.goto_first_child();
    cursor.goto_next_sibling();

    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    cursor.goto_next_sibling();

    debug_assert_eq!(cursor.node().kind(), "object_type");

    let object_type = types::parse(&cursor.node(), ctx).map(Box::new)?;

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Interface(Interface {
            identifier,
            documentation: find_docs(node, ctx.code).map(process_comment),
            object_type,
            exported: is_exported(node),
        }),
        Source::for_node(node, ctx),
    ))
}

fn find_docs<'a>(node: &Node<'a>, code: &'a str) -> Option<&'a str> {
    let parent = node.parent().unwrap();

    if parent.kind() == "export_statement" {
        if let Some(maybe_comment) = parent.prev_sibling() {
            if maybe_comment.kind() == "comment" {
                return Some(maybe_comment.utf8_text(code.as_bytes()).unwrap());
            }
        }
    } else if let Some(maybe_comment) = node.prev_sibling() {
        if maybe_comment.kind() == "comment" {
            return Some(maybe_comment.utf8_text(code.as_bytes()).unwrap());
        }
    }

    None
}

fn is_exported(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        if parent.kind() == "export_statement" {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod test {
    use super::*;
    use dossier_core::tree_sitter::Parser;
    use dossier_core::tree_sitter::TreeCursor;
    use indoc::indoc;
    use std::path::Path;

    fn init_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        parser
    }

    fn walk_tree_to_interface(cursor: &mut TreeCursor) {
        assert_eq!(cursor.node().kind(), "program");
        cursor.goto_first_child();
        loop {
            if cursor.node().kind() == "interface_declaration" {
                break;
            }
            if cursor.node().kind() == "export_statement" {
                cursor.goto_first_child();
                cursor.goto_next_sibling();
                break;
            }

            if !cursor.goto_next_sibling() {
                panic!("Could not find interface_declaration node");
            }
        }
    }

    #[test]
    fn documentation() {
        let code = indoc! {r#"
        /**
         * This is a test interface.
         */
        interface Test {
            test: string;
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert_eq!(
            symbol.kind.as_interface().unwrap().documentation,
            Some("This is a test interface.".to_owned())
        );
    }

    #[test]
    fn exported() {
        let code = indoc! {r#"
        /**
         * This is a test interface.
         */
        export interface Test {
            test: string;
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert!(
            symbol.kind.as_interface().unwrap().exported,
            "Should be exported"
        );
    }
}
