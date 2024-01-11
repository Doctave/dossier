use crate::{
    helpers::*,
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    types, ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Property {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Technically will ever only have one child, the type itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub optional: bool,
    pub readonly: bool,
    pub private: bool,
    pub protected: bool,
}

impl Property {
    pub fn as_entity(
        &self,
        source: &Source,
        fqn: Option<&str>,
        symbol_context: Option<SymbolContext>,
    ) -> Entity {
        let mut meta = json!({});
        if self.optional {
            meta["optional"] = true.into();
        }
        if self.readonly {
            meta["readonly"] = true.into();
        }
        if self.protected {
            meta["protected"] = true.into();
        }
        if self.private {
            meta["private"] = true.into();
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: String::new(),
            kind: "property".to_owned(),
            identity: Identity::FQN(fqn.expect("Parameter without FQN").to_owned()),
            member_context: symbol_context.map(|sc| sc.to_string()),
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
    pub fn the_type(&self) -> Option<&Symbol> {
        self.children.get(0)
    }
}

pub(crate) const NODE_KIND: &str = "property_signature";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = node.walk();
    let mut private = false;
    let mut protected = false;

    let mut children = vec![];

    cursor.goto_first_child();

    while !cursor.node().is_named() {
        cursor.goto_next_sibling();
    }

    if cursor.node().kind() == "accessibility_modifier" {
        match cursor.node().utf8_text(ctx.code.as_bytes()).unwrap() {
            "private" => {
                private = true;
            }
            "protected" => {
                protected = true;
            }
            _ => {}
        }
    }

    let identifier = node
        .child_by_field_name("name")
        .unwrap() // Must have a name
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    cursor.goto_next_sibling();

    // Parse possible type annotation
    if let Some(type_node) = node.child_by_field_name("type") {
        let mut tmp = type_node.walk();
        tmp.goto_first_child();
        tmp.goto_next_sibling();

        ctx.push_scope();
        children.push(types::parse(&tmp.node(), ctx)?);
        ctx.pop_scope();
    }

    let documentation = find_docs(node, ctx.code).map(process_comment);

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Property(Property {
            identifier,
            documentation,
            children,
            private,
            protected,
            readonly: is_readonly(node),
            optional: is_optional(node),
        }),
        Source {
            file: ctx.file.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
        },
    ))
}

fn is_optional(node: &Node) -> bool {
    let mut cursor = node.walk();
    cursor.goto_first_child();
    cursor.goto_next_sibling();

    loop {
        if cursor.node().kind() == "?" {
            return true;
        }
        if !cursor.goto_next_sibling() {
            return false;
        }
    }
}

fn is_readonly(property_node: &Node) -> bool {
    let mut cursor = property_node.walk();

    cursor.goto_first_child();
    loop {
        if cursor.node().kind() == "readonly" {
            return true;
        }
        if !cursor.goto_next_sibling() {
            return false;
        }
    }
}

fn find_docs<'a>(node: &Node<'a>, code: &'a str) -> Option<&'a str> {
    if let Some(maybe_comment) = node.prev_sibling() {
        if maybe_comment.kind() == "comment" {
            return Some(maybe_comment.utf8_text(code.as_bytes()).unwrap());
        }
    }

    None
}

#[cfg(test)]
mod test {
    /// NOTE ABOUT THESE TESTS
    ///
    /// You'll notice that these tests define examples code snippets that
    /// contain an interface. This is because a field can only be defined inside
    /// a class, so we have to construct a class to put the field in it for
    /// the test
    ///
    /// So each test will setup their own context, move the cursor to the
    /// point where the actual type starts, and then parse only the field.
    use super::*;
    use crate::types::Type;
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

    fn walk_tree_to_type(cursor: &mut TreeCursor) {
        cursor.goto_first_child();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        cursor.goto_next_sibling();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
    }

    #[test]
    fn parses_property() {
        let code = indoc! {r#"
            interface Context {
                foo: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();
        assert_eq!(property.identifier, "foo");

        let property_type = property.the_type().unwrap().kind.as_type().unwrap();
        assert_eq!(property_type, &Type::Predefined("number".to_owned()));
    }

    #[test]
    fn parses_property_private_modifier() {
        let code = indoc! {r#"
            interface Context {
                private foo: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert_eq!(property.identifier, "foo");
        assert!(property.private);
        assert!(!property.protected);
        assert!(!property.readonly);
    }

    #[test]
    fn parses_property_protected_modifier() {
        let code = indoc! {r#"
            interface Context {
                protected foo: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert_eq!(property.identifier, "foo");
        assert!(property.protected);
        assert!(!property.private);
        assert!(!property.readonly);
    }

    #[test]
    fn parses_readonly_property() {
        let code = indoc! {r#"
            interface Context {
                readonly foo: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert_eq!(property.identifier, "foo");
        assert!(property.readonly);
        assert!(!property.protected);
        assert!(!property.private);
    }

    #[test]
    fn parses_optional_property() {
        let code = indoc! {r#"
            interface Context {
                foo?: number;
                private readonly bar?: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert!(property.optional);
        assert_eq!(property.identifier, "foo");
        assert!(!property.readonly);
        assert!(!property.protected);
        assert!(!property.private);
    }

    #[test]
    fn parses_optional_property_with_other_modifiers() {
        let code = indoc! {r#"
            interface Context {
                private readonly bar?: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert!(property.optional);
        assert_eq!(property.identifier, "bar");
        assert!(property.readonly);
        assert!(!property.protected);
        assert!(property.private);
    }

    #[test]
    fn parses_property_docs() {
        let code = indoc! {r#"
            interface Context {
                /**
                 * Some documentation
                 */
                readonly foo: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);
        // Walk one extra step because the docs
        cursor.goto_next_sibling();

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let property = symbol.kind.as_property().unwrap();

        assert_eq!(property.identifier, "foo");
        assert_eq!(
            property.documentation,
            Some("Some documentation".to_owned())
        );
    }
}
