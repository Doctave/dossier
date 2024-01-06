use crate::{
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
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
    Object {
        // TODO(Nik): What is the real identifier here?
        raw_string: String,
        properties: Vec<Symbol>,
    },
    Union {
        members: Vec<Symbol>,
    },
}

impl Type {
    /// TODO(Nik): Identifiers don't make sense in this situation
    pub fn identifier(&self) -> &str {
        match self {
            Type::Predefined(type_name) => type_name.as_str(),
            Type::Identifier(identifier, _) => identifier.as_str(),
            Type::Object { raw_string, .. } => raw_string.as_str(),
            Type::Union { .. } => "union",
        }
    }

    pub fn children(&self) -> &[Symbol] {
        match self {
            Type::Object {
                properties: fields, ..
            } => fields,
            Type::Union { members } => members,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self {
            Type::Object {
                properties: ref mut fields,
                ..
            } => fields,
            _ => &mut [],
        }
    }

    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }

    #[cfg(test)]
    pub fn union_left(&self) -> Option<&Symbol> {
        match self {
            Type::Union { members } => members.get(0),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn union_right(&self) -> Option<&Symbol> {
        match self {
            Type::Union { members } => members.get(1),
            _ => None,
        }
    }

    pub fn resolvable_identifier(&self) -> Option<&str> {
        match self {
            Type::Identifier(identifier, _referred_fqn) => Some(identifier.as_str()),
            _ => None,
        }
    }

    pub fn resolve_type(&mut self, fqn: &str) {
        #[allow(clippy::single_match)]
        match self {
            Type::Identifier(_, referred_fqn) => {
                *referred_fqn = Some(fqn.to_owned());
            }
            _ => {}
        }
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    match node.kind() {
        "predefined_type" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Predefined(type_name)),
                Source::for_node(node, ctx),
            ))
        }
        "type_identifier" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Identifier(type_name, None)),
                Source::for_node(node, ctx),
            ))
        }
        "object_type" => {
            let type_as_string = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            let mut properties = vec![];

            ctx.push_context(SymbolContext::Property);

            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            loop {
                if cursor.node().kind() == crate::property::NODE_KIND {
                    let symbol = crate::property::parse(&cursor.node(), ctx)?;
                    properties.push(symbol);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            ctx.pop_scope();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Object {
                    raw_string: type_as_string,
                    properties,
                }),
                Source::for_node(node, ctx),
            ))
        },
        "union_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let left = parse(&cursor.node(), ctx)?;

            cursor.goto_next_sibling();
            debug_assert_eq!(cursor.node().kind(), "|");
            cursor.goto_next_sibling();

            let right = parse(&cursor.node(), ctx)?;

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Union {
                    members: vec![left, right],
                }),
                Source::for_node(node, ctx),
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
mod test {
    /// NOTE ABOUT THESE TESTS
    ///
    /// You'll notice that these tests define examples code snippets that
    /// contain a type alias (e.g. `type Foo = string;`). This is because
    /// a type definition is not valid on its own, and the parser will fail.
    /// We need the type to be in some kind of context.
    ///
    /// So each test will setup their own context, move the cursor to the
    /// point where the actual type starts, and then parse only the type.
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

    fn walk_tree_to_type(cursor: &mut TreeCursor) {
        cursor.goto_first_child();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        cursor.goto_next_sibling();
        cursor.goto_next_sibling();
    }

    #[test]
    fn parses_predefined_type() {
        let code = indoc! {r#"
            type Foo = string;
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

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Predefined(type_name) => {
                assert_eq!(type_name, "string");
            }
            _ => panic!("Expected a predefined type"),
        }
    }

    #[test]
    fn parses_type_identifiers() {
        let code = indoc! {r#"
            type Foo = Bar;
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

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Identifier(type_name, referred_fqn) => {
                assert_eq!(type_name, "Bar");
                assert_eq!(
                    referred_fqn, &None,
                    "The type should not be resolved at this point"
                );
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_object_types() {
        let code = indoc! {r#"
            type Player = {
                name: string;
                age?: number;
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

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Object { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_eq!(properties[0].kind.as_property().unwrap().identifier, "name");
                assert!(!properties[0].kind.as_property().unwrap().is_optional);
                assert_eq!(
                    properties[0].kind.as_property().unwrap().children[0]
                        .kind
                        .as_type()
                        .unwrap(),
                    &Type::Predefined("string".to_string())
                );
                assert_eq!(properties[1].kind.as_property().unwrap().identifier, "age");
                assert!(properties[1].kind.as_property().unwrap().is_optional);
                assert_eq!(
                    properties[1].kind.as_property().unwrap().children[0]
                        .kind
                        .as_type()
                        .unwrap(),
                    &Type::Predefined("number".to_string())
                );
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_union_type() {
        let code = indoc! {r#"
            type Foo = string | number;
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

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Union { .. } => {
                let left = type_def.union_left().unwrap().kind.as_type().unwrap();
                assert_eq!(left, &Type::Predefined("string".to_string()));

                let right = type_def.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(right, &Type::Predefined("number".to_string()));
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_nested_union_type() {
        let code = indoc! {r#"
            type Foo = string | number | boolean;
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

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Union { .. } => {
                let left = type_def.union_left().unwrap().kind.as_type().unwrap();
                let left_left = left.union_left().unwrap().kind.as_type().unwrap();
                assert_eq!(left_left, &Type::Predefined("string".to_string()));

                let left_right = left.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(left_right, &Type::Predefined("number".to_string()));

                let right = type_def.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(right, &Type::Predefined("boolean".to_string()));
            }
            _ => panic!("Expected a type identifier"),
        }
    }
}
