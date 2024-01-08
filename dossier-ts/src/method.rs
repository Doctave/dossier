use crate::{
    helpers::*,
    parameter,
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    type_variable, types, ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Entity, Identity, Result};

use indoc::indoc;
use lazy_static::lazy_static;

const QUERY_STRING: &str = indoc! {"
    [
        (method_definition 
            name: [(property_identifier) (private_property_identifier) (computed_property_name)] @method_name
            type_parameters: (type_parameters) ? @method_type_parameters
            parameters: (formal_parameters) @method_parameters
            return_type: (type_annotation) ? @method_return_type
        ) @method
        (method_signature 
            name: [(property_identifier) (private_property_identifier) (computed_property_name)] @method_name
            type_parameters: (type_parameters) ? @method_type_parameters
            parameters: (formal_parameters) @method_parameters
            return_type: (type_annotation) ? @method_return_type
        ) @method
        (abstract_method_signature 
            name: [(property_identifier) (private_property_identifier) (computed_property_name)] @method_name
            type_parameters: (type_parameters) ? @method_type_parameters
            parameters: (formal_parameters) @method_parameters
            return_type: (type_annotation) ? @method_return_type
        ) @method
    ]
    "};

lazy_static! {
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

pub(crate) const NODE_KIND: &str = "method_signature";

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Identifier {
    Computed(String),
    Name(String),
}

impl Identifier {
    pub fn as_str(&self) -> &str {
        match self {
            Identifier::Computed(s) => s.as_str(),
            Identifier::Name(s) => s.as_str(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Method {
    pub identifier: Identifier,
    pub children: Vec<Symbol>,
    pub documentation: Option<String>,
    pub is_abstract: bool,
    pub is_private: bool,
}

impl Method {
    pub fn as_entity(&self, source: &Source, fqn: Option<&str>) -> Entity {
        let mut meta = json!({});

        if self.is_abstract {
            meta["abstract"] = true.into();
        }

        Entity {
            title: Some(self.identifier.as_str().to_owned()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "method".to_owned(),
            identity: Identity::FQN(fqn.expect("Method without FQN").to_owned()),
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
    #[allow(dead_code)]
    pub fn parameters(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_parameter().is_some())
    }

    #[cfg(test)]
    #[allow(dead_code)]
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

/// TODO(Nik): This code is almost identical to the code in function.rs. We
/// should try to find a way to share this code and test it in one place.
pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert!(matches!(
        node.kind(),
        "method_signature" | "method_definition" | "abstract_method_signature"
    ));

    let mut children = vec![];

    let mut cursor = QueryCursor::new();
    let method = cursor
        .matches(&QUERY, *node, ctx.code.as_bytes())
        .next()
        .unwrap();

    let main_node = node_for_capture("method", method.captures, &QUERY).unwrap();
    let name_node = node_for_capture("method_name", method.captures, &QUERY).unwrap();
    let type_param_node = node_for_capture("method_type_parameters", method.captures, &QUERY);
    let parameters_node = node_for_capture("method_parameters", method.captures, &QUERY);
    let return_type_node = node_for_capture("method_return_type", method.captures, &QUERY);

    let identifier = if name_node.kind() == "computed_property_name" {
        let mut cursor = name_node.walk();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        Identifier::Computed(cursor.node().utf8_text(ctx.code.as_bytes()).unwrap().to_owned())
    } else {
        Identifier::Name(name_node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned())
    };

    ctx.push_scope();
    ctx.push_fqn(identifier.as_str());

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
        SymbolKind::Method(Method {
            identifier,
            documentation: docs.map(process_comment),
            children,
            is_abstract: node.kind() == "abstract_method_signature",
            is_private: name_node.kind() == "private_property_identifier",
        }),
        Source::for_node(&main_node, ctx),
    ))
}

fn parse_return_type(
    node: &Node,
    children: &mut Vec<Symbol>,
    ctx: &mut ParserContext,
) -> Result<()> {
    let mut type_node_cursor = node.walk();
    type_node_cursor.goto_first_child();
    while !type_node_cursor.node().is_named() {
        type_node_cursor.goto_next_sibling();
    }
    ctx.push_context(SymbolContext::ReturnType);
    children.push(types::parse(&type_node_cursor.node(), ctx).unwrap());
    ctx.pop_context();
    Ok(())
}

fn parse_parameters(
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
            let parameter = parameter::parse(&cursor.node(), ctx)?;
            children.push(parameter);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
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
