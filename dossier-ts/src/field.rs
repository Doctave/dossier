use crate::{
    helpers::*,
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};
use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Field {
    pub identifier: String,
    /// Technically will ever only have max one child, the value itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub readonly: bool,
    pub private: bool,
    pub protected: bool,
    pub documentation: Option<String>,

    /// For now, we're going to just parse a value as a string literal.
    /// This is because it's essentially arbitrary code, and we don't want to
    /// parse it as a full expression.
    ///
    /// We may want to parse out the simple cases like string and number
    /// constants in the future, but for now we'll just leave it as a string.
    pub value: Option<String>,
}

impl Field {
    pub fn as_entity(&self, source: &Source, fqn: Option<&str>) -> Entity {
        let mut meta = json!({});
        if self.readonly {
            meta["readonly"] = true.into();
        }
        if self.protected {
            meta["protected"] = true.into();
        }
        if self.private {
            meta["private"] = true.into();
        }
        if let Some(value) = &self.value {
            meta["value"] = json!(value);
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "field".to_owned(),
            identity: Identity::FQN(fqn.expect("Field did not have FQN").to_owned()),
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
    pub fn the_type(&self) -> Option<&Symbol> {
        self.children.iter().find(|s| s.kind.as_type().is_some())
    }
}

pub(crate) const NODE_KIND: &str = "public_field_definition";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut value = None;
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

        children.push(types::parse(&tmp.node(), ctx)?);
    }

    // Parse possible value
    if let Some(value_node) = node.child_by_field_name("value") {
        value = Some(
            value_node
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned(),
        );
    }

    let documentation = find_docs(node, ctx.code).map(process_comment);

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Field(Field {
            identifier,
            children,
            readonly: is_readonly(node),
            documentation,
            private,
            protected,
            value,
        }),
        Source {
            file: ctx.file.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
        },
    ))
}

fn is_readonly(field_node: &Node) -> bool {
    let mut cursor = field_node.walk();

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
    /// contain a class. This is because a field can only be defined inside
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
    fn parses_field() {
        // NOTE: This example technically required  @strictPropertyInitialization: false
        //
        // Example: https://www.typescriptlang.org/play?strictPropertyInitialization=false#code/PTAEAEGcBcCcEsDG0AKsD2AHApraBPASQDt5p4BDAG3gC8Lz1iAuUAM2smwFgAoRKhUiRQKdPGLRQAbz6hQAD1bEArgFsARrgDcc0PmXqtsXbwC+fPoiYxQmKQF5QxbAHdR4yQAoAlKfsAdAqgTgAM-tAB+CGg4UA
        let code = indoc! {r#"
            class Context {
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

        let field = symbol.kind.as_field().unwrap();
        assert_eq!(field.identifier, "foo");

        let field_type = field.the_type().unwrap().kind.as_type().unwrap();
        assert_eq!(field_type, &Type::Predefined("number".to_owned()));
    }

    #[test]
    fn parses_field_with_number_value() {
        let code = indoc! {r#"
            class Context {
                foo = 123;
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert_eq!(field.value.as_ref().unwrap(), "123");
    }

    #[test]
    fn parses_field_with_string_value() {
        let code = indoc! {r#"
            class Context {
                foo = "an string";
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert_eq!(field.value.as_ref().unwrap(), "\"an string\"");
    }

    #[test]
    fn parses_field_with_expression() {
        let code = indoc! {r#"
            class Context {
                foo = new Bar();
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert_eq!(field.value.as_ref().unwrap(), "new Bar()");
    }

    #[test]
    fn parses_field_private_modifier() {
        let code = indoc! {r#"
            class Context {
                private foo: number = 123;
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert!(field.private);
        assert!(!field.protected);
        assert!(!field.readonly);
    }

    #[test]
    fn parses_field_protected_modifier() {
        let code = indoc! {r#"
            class Context {
                protected foo: number = 123;
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert!(field.protected);
        assert!(!field.private);
        assert!(!field.readonly);
    }

    #[test]
    fn parses_readonly_field() {
        let code = indoc! {r#"
            class Context {
                readonly foo: number = 123;
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert!(field.readonly);
        assert!(!field.protected);
        assert!(!field.private);
    }

    #[test]
    fn parses_field_docs() {
        let code = indoc! {r#"
            class Context {
                /**
                 * Some documentation
                 */
                readonly foo: number = 123;
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

        let field = symbol.kind.as_field().unwrap();

        assert_eq!(field.identifier, "foo");
        assert_eq!(field.documentation, Some("Some documentation".to_owned()));
    }
}
