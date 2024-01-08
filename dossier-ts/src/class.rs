use crate::{
    field,
    helpers::*,
    method,
    symbol::{Source, Symbol, SymbolKind},
    ParserContext,
};
use dossier_core::{serde_json::json, tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "class_declaration";
pub(crate) const ABSTRACT_NODE_KIND: &str = "abstract_class_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Class {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Interfaces are actually just a single object type.
    /// We forward a bunch of methods to this child object.
    pub children: Vec<Symbol>,
    pub exported: bool,
    pub is_abstract: bool,
}

impl Class {
    pub(crate) fn as_entity(&self, source: &Source, fqn: Option<&str>) -> Entity {
        let mut meta = json!({});
        if self.exported {
            meta["exported"] = true.into();
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "class".to_owned(),
            identity: Identity::FQN(fqn.expect("Class did not have FQN").to_owned()),
            member_context: None,
            language: "ts".to_owned(),
            source: source.as_entity_source(),
            meta,
            members: self
                .children
                .iter()
                .map(|s| s.as_entity())
                .collect::<Vec<_>>(),
        }
    }

    #[cfg(test)]
    pub(crate) fn fields(&self) -> impl Iterator<Item = &Symbol> {
        self.children.iter().filter(|s| s.kind.as_field().is_some())
    }

    #[cfg(test)]
    pub(crate) fn methods(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_method().is_some())
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert!(
        matches!(node.kind(), NODE_KIND | ABSTRACT_NODE_KIND),
        "Expected node kind to be either {} or {}, but was {}",
        NODE_KIND,
        ABSTRACT_NODE_KIND,
        node.kind()
    );

    let mut is_abstract = false;

    if node.kind() == ABSTRACT_NODE_KIND {
        is_abstract = true;
    }

    let mut children = vec![];

    let identifier = node
        .child_by_field_name("name")
        .unwrap() // Must have a name
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    ctx.push_scope();
    ctx.push_fqn(&identifier);

    parse_class_body(
        &node.child_by_field_name("body").unwrap(),
        ctx,
        &mut children,
    )?;

    ctx.pop_fqn();
    ctx.pop_scope();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Class(Class {
            identifier,
            documentation: find_docs(node, ctx.code).map(process_comment),
            children,
            exported: is_exported(node),
            is_abstract,
        }),
        Source::for_node(node, ctx),
    ))
}

fn parse_class_body(
    node: &Node,
    ctx: &mut ParserContext,
    children: &mut Vec<Symbol>,
) -> Result<()> {
    let mut cursor = node.walk();

    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "method_definition" {
            children.push(method::parse(&cursor.node(), ctx)?);
        }
        if cursor.node().kind() == "abstract_method_signature" {
            children.push(method::parse(&cursor.node(), ctx)?);
        }
        if cursor.node().kind() == field::NODE_KIND {
            children.push(field::parse(&cursor.node(), ctx)?);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
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
    use dossier_core::tree_sitter::Parser;
    use dossier_core::tree_sitter::TreeCursor;
    use indoc::indoc;
    use std::path::Path;

    use super::*;

    fn init_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        parser
    }

    fn walk_tree_to_class(cursor: &mut TreeCursor) {
        assert_eq!(cursor.node().kind(), "program");
        cursor.goto_first_child();
        loop {
            if cursor.node().kind() == "class_declaration" {
                break;
            }
            if cursor.node().kind() == "abstract_class_declaration" {
                break;
            }
            if cursor.node().kind() == "export_statement" {
                cursor.goto_first_child();
                cursor.goto_next_sibling();
                break;
            }

            if !cursor.goto_next_sibling() {
                panic!("Could not find class_declaration node");
            }
        }
    }

    #[test]
    fn abstract_class() {
        let code = indoc! { r#"
            abstract class Base {
                abstract getName(): string;
            }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_class(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert_eq!(symbol.kind.as_class().unwrap().identifier, "Base");
        assert!(symbol.kind.as_class().unwrap().is_abstract);

        let abstract_method = symbol
            .kind
            .as_class()
            .unwrap()
            .children
            .iter()
            .find(|s| s.kind.as_method().is_some())
            .unwrap();

        assert!(abstract_method.kind.as_method().unwrap().is_abstract);
    }

    #[test]
    fn private_method_identifier() {
        let code = indoc! {r#"
        class Example {
          #privateMethod() {}
        }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_class(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        // Parse successfully
        let _symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let private_method = symbol
            .kind
            .as_class()
            .unwrap()
            .children
            .iter()
            .find(|s| s.kind.as_method().is_some())
            .unwrap();

        assert!(private_method.kind.as_method().unwrap().is_private);
    }

    #[test]
    fn computed_method_identifier() {
        let code = indoc! {r#"
        class Example {
          [SOME_IDENTIFIER]() {}
        }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_class(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        // Parse successfully
        let _symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let method = symbol
            .kind
            .as_class()
            .unwrap()
            .children
            .iter()
            .find(|s| s.kind.as_method().is_some())
            .unwrap();

        let identifier = &method.kind.as_method().unwrap().identifier;
        assert_eq!(
            identifier,
            &crate::method::Identifier::Computed("SOME_IDENTIFIER".into())
        );
    }
}
