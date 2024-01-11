use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Entity, Identity, Result};
use indoc::indoc;
use lazy_static::lazy_static;

use crate::{helpers::*, parameter, type_variable};
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
    pub fn as_entity(
        &self,
        source: &Source,
        fqn: Option<&str>,
        symbol_context: Option<SymbolContext>,
    ) -> Entity {
        let mut meta = json!({});
        if self.is_exported {
            meta["exported"] = true.into();
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "function".to_owned(),
            identity: Identity::FQN(fqn.expect("Function did not have FQN").to_owned()),
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
    pub fn parameters(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_parameter().is_some())
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
    let type_param_node = node_for_capture("function_type_parameters", function.captures, &QUERY);
    let parameters_node = node_for_capture("function_parameters", function.captures, &QUERY);
    let return_type_node = node_for_capture("function_return_type", function.captures, &QUERY);

    let identifier = name_node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();

    ctx.push_scope();
    ctx.push_fqn(&identifier);

    if let Some(type_parameters) = type_param_node {
        parse_type_parameters(&type_parameters, &mut children, ctx);
        ctx.push_scope();
    }

    if let Some(parameter_nodes) = parameters_node {
        parse_parameters(&parameter_nodes, &mut children, ctx)?;
    }

    if let Some(type_node) = return_type_node {
        parse_return_type(&type_node, &mut children, ctx)?;
    }

    let docs = find_docs(&main_node, ctx.code);

    if type_param_node.is_some() {
        ctx.pop_scope();
    }
    ctx.pop_scope();
    ctx.pop_fqn();

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

pub(crate) fn parse_return_type(
    node: &Node,
    children: &mut Vec<Symbol>,
    ctx: &mut ParserContext,
) -> Result<()> {
    let mut type_node_cursor = node.walk();
    type_node_cursor.goto_first_child();
    while !type_node_cursor.node().is_named() {
        type_node_cursor.goto_next_sibling();
    }
    let mut the_type = types::parse(&type_node_cursor.node(), ctx).unwrap();
    the_type.context = Some(SymbolContext::ReturnType);
    children.push(the_type);
    Ok(())
}

pub(crate) fn parse_parameters(
    parameters: &Node,
    children: &mut Vec<Symbol>,
    ctx: &mut ParserContext,
) -> Result<()> {
    assert_eq!(parameters.kind(), "formal_parameters");

    let mut cursor = parameters.walk();
    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "required_parameter"
            || cursor.node().kind() == "optional_parameter"
        {
            let mut parameter = parameter::parse(&cursor.node(), ctx)?;
            parameter.context = Some(SymbolContext::Parameter);
            children.push(parameter);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
}

pub(crate) fn parse_type_parameters(
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
    fn fqns() {
        let code = indoc! {r#"
        function foo<Bar extends Baz>() {}
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_function(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert_eq!(symbol.fqn.unwrap(), "index.ts::foo");

        let type_variable = symbol
            .kind
            .as_function()
            .unwrap()
            .type_variables()
            .next()
            .unwrap();
        assert_eq!(type_variable.fqn.as_ref().unwrap(), "index.ts::foo::Bar");
    }

    #[test]
    fn parameters() {
        let code = indoc! {r#"
        function foo(bar: string, baz, fizz?) {}
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

        let params = function.parameters().collect::<Vec<_>>();
        assert_eq!(params.len(), 3);
        assert!(params
            .iter()
            .all(|p| p.context == Some(SymbolContext::Parameter)));

        let bar = params[0].kind.as_parameter().unwrap();
        assert_eq!(params[0].context, Some(SymbolContext::Parameter));
        assert_eq!(bar.identifier, "bar");
        assert!(!bar.optional);
        assert_eq!(
            bar.parameter_type().unwrap().kind.as_type().unwrap(),
            &Type::Predefined("string".to_owned())
        );

        let baz = params[1].kind.as_parameter().unwrap();
        assert_eq!(baz.identifier, "baz");
        assert!(!baz.optional);
        assert_eq!(baz.parameter_type(), None);

        let fizz = params[2].kind.as_parameter().unwrap();
        assert_eq!(fizz.identifier, "fizz");
        assert!(fizz.optional);
        assert_eq!(fizz.parameter_type(), None);
    }

    #[test]
    fn readonly_parameter() {
        let code = indoc! {r#"
        function foo(bar: readonly string) {}
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

        let params = function.parameters().collect::<Vec<_>>();
        assert_eq!(params.len(), 1);

        let bar = params[0].kind.as_parameter().unwrap();
        assert_eq!(bar.identifier, "bar");
        assert!(bar.readonly);
        assert!(!bar.optional);
        assert_eq!(
            bar.parameter_type().unwrap().kind.as_type().unwrap(),
            &Type::Predefined("string".to_owned())
        );
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

        // check the type variable
        assert_eq!(function.type_variables().count(), 1);

        let type_variable = function.type_variables().collect::<Vec<_>>()[0];
        assert!(symbol.scope_id < type_variable.scope_id);

        let type_variable_kind = type_variable.kind.as_type_variable().unwrap();
        assert_eq!(type_variable_kind.identifier, "Type");
        assert_eq!(type_variable_kind.constraints().count(), 1);

        // check the type variable's constraint
        let constraint = type_variable_kind.constraints().next().unwrap();

        let constraint_kind = constraint.kind.as_type_constraint().unwrap();

        assert!(constraint_kind.extends);
        assert_eq!(
            constraint_kind
                .the_type()
                .kind
                .as_type()
                .unwrap()
                .identifier()
                .unwrap(),
            "SomeOtherType"
        );

        // Verify that the return type has a scope that is larger than the type variable's scope
        let return_type = function.return_type().unwrap();
        assert!(
            type_variable.scope_id < return_type.scope_id,
            "Expected type variable scope to be smaller than return type scope: {:?} < {:?}",
            type_variable.scope_id,
            return_type.scope_id
        );
    }

    #[test]
    fn generics_with_key_of_constraint() {
        let code = indoc! {r#"
        function example<A extends keyof B>() {}
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
        assert_eq!(function.identifier, "example");

        // check the type variable
        assert_eq!(function.type_variables().count(), 1);

        let type_variable = function.type_variables().collect::<Vec<_>>()[0];
        assert!(symbol.scope_id < type_variable.scope_id);

        let type_variable_kind = type_variable.kind.as_type_variable().unwrap();
        assert_eq!(type_variable_kind.identifier, "A");
        assert_eq!(type_variable_kind.constraints().count(), 1);

        // check the type variable's constraint
        let constraint = type_variable_kind.constraints().next().unwrap();

        let constraint_kind = constraint.kind.as_type_constraint().unwrap();

        assert!(constraint_kind.extends);
        let type_kind = constraint_kind.the_type().kind.as_type().unwrap();
        assert!(matches!(type_kind, &Type::KeyOf(_)));
    }
}
