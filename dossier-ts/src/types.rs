use crate::{
    function, method,
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

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
    GenericType {
        identifier: String,
        members: Vec<Symbol>,
    },
    Array {
        members: Vec<Symbol>,
    },
    Function {
        members: Vec<Symbol>,
    },
    TypeOf(String),
}

impl Type {
    /// TODO(Nik): Identifiers don't make sense in this situation
    pub fn identifier(&self) -> &str {
        match self {
            Type::Predefined(type_name) => type_name.as_str(),
            Type::Identifier(identifier, _) => identifier.as_str(),
            Type::Object { raw_string, .. } => raw_string.as_str(),
            Type::Union { .. } => "union",
            Type::GenericType { identifier, .. } => identifier.as_str(),
            Type::Array { .. } => "array",
            Type::Function { .. } => "function",
            // TODO(Nik): Does this make sense?
            Type::TypeOf(name) => name,
        }
    }

    pub fn children(&self) -> &[Symbol] {
        match self {
            Type::Object {
                properties: fields, ..
            } => fields,
            Type::Union { members } => members,
            Type::GenericType { members, .. } => members,
            Type::Array { members, .. } => members,
            Type::Function { members, .. } => members,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self {
            Type::Object {
                properties: ref mut fields,
                ..
            } => fields,
            Type::Union { ref mut members } => members,
            Type::GenericType {
                ref mut members, ..
            } => members,
            Type::Array {
                ref mut members, ..
            } => members,
            Type::Function {
                ref mut members, ..
            } => members,
            _ => &mut [],
        }
    }

    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        match &self {
            Type::TypeOf(name) => {
                let meta = json!({});

                Entity {
                    title: format!("typeof {}", name),
                    description: String::new(),
                    kind: "typeof".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Function { members } => {
                let meta = json!({});

                Entity {
                    title: "function_type".to_owned(),
                    description: String::new(),
                    kind: "function_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Array { members } => {
                let meta = json!({});

                Entity {
                    title: "array_type".to_owned(),
                    description: String::new(),
                    kind: "array_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::GenericType {
                identifier,
                members,
            } => {
                let meta = json!({});

                Entity {
                    title: identifier.to_owned(),
                    description: String::new(),
                    kind: "generic_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Union { .. } => {
                let meta = json!({});

                Entity {
                    title: "union".to_owned(),
                    description: String::new(),
                    kind: "union".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![
                        self.union_left().unwrap().as_entity(),
                        self.union_right().unwrap().as_entity(),
                    ],
                }
            }
            Type::Object { .. } => {
                let meta = json!({});

                Entity {
                    title: "object".to_owned(),
                    description: String::new(),
                    kind: "object".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: self.children().iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Predefined(type_name) => {
                let meta = json!({});

                Entity {
                    title: type_name.clone(),
                    description: String::new(),
                    kind: "predefined_type".to_owned(),
                    identity: Identity::FQN(format!("builtin::{}", type_name)),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Identifier(type_name, reference) => {
                let meta = json!({});

                Entity {
                    title: type_name.clone(),
                    description: String::new(),
                    kind: "predefined_type".to_owned(),
                    identity: if let Some(fqn) = reference {
                        Identity::Reference(fqn.to_owned())
                    } else {
                        Identity::FQN(fqn.to_owned())
                    },
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
        }
    }

    #[cfg(test)]
    pub fn function_parameters(&self) -> impl Iterator<Item = &Symbol> {
        match &self {
            Type::Function { members } => {
                members.iter().filter(|s| s.kind.as_parameter().is_some())
            }
            _ => panic!("Expected a function type"),
        }
    }

    #[cfg(test)]
    pub fn function_return_type(&self) -> Option<&Symbol> {
        match &self {
            Type::Function { members } => members
                .iter()
                .find(|s| s.context == Some(crate::symbol::SymbolContext::ReturnType)),
            _ => panic!("Expected a function type"),
        }
    }

    pub fn union_left(&self) -> Option<&Symbol> {
        match self {
            Type::Union { members } => members.get(0),
            _ => None,
        }
    }

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
        "type_query" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            let identifier = cursor
                .node()
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::TypeOf(identifier)),
                Source::for_node(node, ctx),
            ))
        }
        "function_type" => {
            let mut members = vec![];

            if let Some(params) = node.child_by_field_name("parameters") {
                function::parse_parameters(&params, &mut members, ctx)?;
            }
            if let Some(params) = node.child_by_field_name("return_type") {
                ctx.push_context(SymbolContext::ReturnType);
                members.push(parse(&params, ctx)?);
                ctx.pop_context()
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Function { members }),
                Source::for_node(node, ctx),
            ))
        }
        "array_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();

            members.push(parse(&cursor.node(), ctx)?);

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Array { members }),
                Source::for_node(node, ctx),
            ))
        }
        "generic_type" => {
            let identifier = node
                .child_by_field_name("name")
                .unwrap()
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned();

            let mut members = vec![];
            for arg in node.children_by_field_name("type_arguments", &mut node.walk()) {
                let mut cursor = arg.walk();
                cursor.goto_first_child();
                cursor.goto_next_sibling();

                ctx.push_fqn(&identifier);

                members.push(parse(&cursor.node(), ctx)?);

                ctx.pop_fqn();
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::GenericType {
                    identifier,
                    members,
                }),
                Source::for_node(node, ctx),
            ))
        }
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
                if cursor.node().kind() == method::NODE_KIND {
                    let symbol = method::parse(&cursor.node(), ctx)?;
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
        }
        "union_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let mut members = vec![];

            loop {
                if cursor.node().kind() == "|" {
                    cursor.goto_next_sibling();
                    continue;
                }
                members.push(parse(&cursor.node(), ctx)?);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Union { members }),
                Source::for_node(node, ctx),
            ))
        }
        _ => panic!(
            "Unhandled type kind: {} | {} | {} | file:{} | pos:{}",
            node.kind(),
            node.utf8_text(ctx.code.as_bytes()).unwrap(),
            node.to_sexp(),
            ctx.file.display(),
            node.start_position()
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
                assert!(!properties[0].kind.as_property().unwrap().optional);
                assert_eq!(
                    properties[0].kind.as_property().unwrap().children[0]
                        .kind
                        .as_type()
                        .unwrap(),
                    &Type::Predefined("string".to_string())
                );
                assert_eq!(properties[1].kind.as_property().unwrap().identifier, "age");
                assert!(properties[1].kind.as_property().unwrap().optional);
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

    #[test]
    fn parses_generic_type() {
        let code = indoc! {r#"
            type Foo = Promise<Example>;
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

        assert_eq!(type_def.identifier(), "Promise");
        assert_eq!(type_def.children().len(), 1);
        assert!(matches!(type_def, Type::GenericType { .. }));

        let arg = type_def.children()[0].kind.as_type().unwrap();
        assert_eq!(arg, &Type::Identifier("Example".to_owned(), None));
    }

    #[test]
    fn parses_array_type() {
        let code = indoc! {r#"
            type Foo = string[];
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

        assert!(matches!(type_def, Type::Array { .. }));

        assert_eq!(type_def.children().len(), 1);

        let arg = type_def.children()[0].kind.as_type().unwrap();
        assert_eq!(arg, &Type::Predefined("string".to_owned()));
    }

    #[test]
    fn parses_function_type() {
        let code = indoc! {r#"
            type Foo = (a: string) => void;
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

        assert!(matches!(type_def, Type::Function { .. }));

        assert_eq!(type_def.children().len(), 2);

        let param = type_def.function_parameters().next().unwrap();
        assert_eq!(param.kind.as_parameter().unwrap().identifier, "a");

        let return_type = type_def.function_return_type().unwrap();
        assert_eq!(
            return_type.kind.as_type().unwrap(),
            &Type::Predefined("void".to_owned())
        );
    }

    #[test]
    fn parses_typeof() {
        let code = indoc! {r#"
            type Request = typeof TediousRequest
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

        assert_eq!(type_def, &Type::TypeOf("TediousRequest".to_owned()));
    }

    #[test]
    fn bug_unbalanced_union() {
        let code = indoc! {r#"
            type Example = | number
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

        assert!(matches!(type_def, Type::Union { .. }));

        assert_eq!(type_def.children().len(), 1);
    }
}
