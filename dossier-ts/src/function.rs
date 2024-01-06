use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Entity, Result};
use indoc::indoc;
use lazy_static::lazy_static;

use crate::{helpers::*, type_variable};
use crate::{
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    types, ParserContext,
};

const QUERY_STRING: &str = indoc! {"
    (function_declaration 
          name: (identifier) @function_name
          type_parameters: (type_parameters) ? @function_type_parameters
          parameters: (formal_parameters) @function_parameters
          return_type: (type_annotation) ? @function_return_type
    ) @function
    "};

lazy_static! {
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

pub(crate) const NODE_KIND: &str = "function_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function {
    pub identifier: String,
    pub documentation: Option<String>,
    pub is_exported: bool,
    pub children: Vec<Symbol>,
}

impl Function {
    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        Entity {
            title: self.identifier.clone(),
            description: self.documentation.clone().unwrap_or_default(),
            kind: "function".to_owned(),
            identity: dossier_core::Identity::FQN(fqn.to_owned()),
            members: vec![],
            member_context: None,
            language: crate::LANGUAGE.to_owned(),
            source: dossier_core::Source {
                file: source.file.to_owned(),
                start_offset_bytes: source.offset_start_bytes,
                end_offset_bytes: source.offset_end_bytes,
                repository: None,
            },
            meta: json!({}),
        }
    }

    #[cfg(test)]
    pub fn type_variables(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_type_variable().is_some())
    }

    #[cfg(test)]
    pub fn return_type(&self) -> Option<&Symbol> {
        self.children
            .iter()
            .find(|s| s.context == Some(crate::symbol::SymbolContext::ReturnType))
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut children = vec![];

    let mut cursor = QueryCursor::new();
    let function = cursor
        .matches(&QUERY, *node, ctx.code.as_bytes())
        .next()
        .unwrap();

    let main_node = node_for_capture("function", function.captures, &QUERY).unwrap();
    let name_node = node_for_capture("function_name", function.captures, &QUERY).unwrap();

    let identifier = name_node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();

    ctx.push_scope(identifier.as_str());

    // let parameter_node = node_for_capture("function_parameters", m.captures, &QUERY);
    if let Some(type_node) = node_for_capture("function_return_type", function.captures, &QUERY) {
        let mut type_node_cursor = type_node.walk();
        type_node_cursor.goto_first_child();
        while !type_node_cursor.node().is_named() {
            type_node_cursor.goto_next_sibling();
        }
        ctx.push_context(SymbolContext::ReturnType);
        children.push(types::parse(&type_node_cursor.node(), ctx).unwrap());
        ctx.pop_context();
    }

    if let Some(type_parameters) =
        node_for_capture("function_type_parameters", function.captures, &QUERY)
    {
        parse_type_parameters(&type_parameters, &mut children, ctx);
    }

    let docs = find_docs(&main_node, ctx.code);

    ctx.pop_scope();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Function(Function {
            identifier,
            documentation: docs.map(process_comment),
            is_exported: is_exported(&main_node),
            children,
        }),
        Source::for_node(&main_node, ctx),
    ))
}

fn parse_type_parameters(
    type_parameters: &Node,
    children: &mut Vec<Symbol>,
    ctx: &mut ParserContext,
) {
    assert_eq!(type_parameters.kind(), "type_parameters");

    let mut cursor = type_parameters.walk();
    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "type_parameter" {
            let type_variable = type_variable::parse(&cursor.node(), ctx).unwrap();
            children.push(type_variable);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
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
    use crate::types::Type;

    use super::*;
    use dossier_core::tree_sitter::Parser;
    use dossier_core::tree_sitter::TreeCursor;
    use std::path::Path;

    fn init_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        parser
    }

    fn walk_tree_to_function(cursor: &mut TreeCursor) {
        assert_eq!(cursor.node().kind(), "program");
        cursor.goto_first_child();
        assert_eq!(cursor.node().kind(), "function_declaration");
    }

    #[test]
    fn generics() {
        let code = indoc! {r#"
        function identity<Type>(arg: Type): Type {}
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_function(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let function = symbol.kind.as_function().unwrap();

        assert_eq!(function.identifier, "identity");
        assert_eq!(function.type_variables().count(), 1);

        let type_variable = function.type_variables().collect::<Vec<_>>()[0];
        assert!(type_variable.scope_id > symbol.scope_id);

        let type_variable_kind = type_variable.kind.as_type_variable().unwrap();
        assert_eq!(type_variable_kind.identifier, "Type");
        assert_eq!(type_variable_kind.constraints().count(), 0);

        let return_type = function.return_type().unwrap().kind.as_type().unwrap();
        match return_type {
            Type::Identifier(identifier, None) => {
                assert_eq!(identifier, "Type");
            }
            _ => panic!("Expected type variable"),
        }
    }

    #[test]
    fn generics_with_extends_constraint() {
        let code = indoc! {r#"
        function identity<Type extends SomeOtherType>(arg: Type): Type {}
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_function(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let function = symbol.kind.as_function().unwrap();

        assert_eq!(function.identifier, "identity");
        assert_eq!(function.type_variables().count(), 1);

        let type_variable = function.type_variables().collect::<Vec<_>>()[0];
        assert!(type_variable.scope_id > symbol.scope_id);

        let type_variable_kind = type_variable.kind.as_type_variable().unwrap();
        assert_eq!(type_variable_kind.identifier, "Type");
        assert_eq!(type_variable_kind.constraints().count(), 1);

        let constraint = type_variable_kind.constraints().next().unwrap();

        let constraint_kind = constraint.kind.as_type_constraint().unwrap();

        assert!(constraint_kind.extends);
        assert_eq!(
            constraint_kind
                .the_type()
                .kind
                .as_type()
                .unwrap()
                .identifier(),
            "SomeOtherType"
        );
    }
}
