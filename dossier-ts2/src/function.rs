use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Entity, Result};
use indoc::indoc;
use lazy_static::lazy_static;

use crate::helpers::*;
use crate::symbol::SymbolContext;
use crate::{
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
};

const QUERY_STRING: &str = indoc! {"
    (function_declaration 
          name: (identifier) @function_name
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

    let docs = find_docs(&main_node, ctx.code);

    ctx.pop_scope();

    Ok(Symbol {
        fqn: ctx.construct_fqn(&identifier),
        kind: SymbolKind::Function(Function {
            identifier,
            documentation: docs.map(process_comment),
            is_exported: is_exported(&main_node),
            children,
        }),
        source: Source {
            file: ctx.file.to_owned(),
            offset_start_bytes: main_node.start_byte(),
            offset_end_bytes: main_node.end_byte(),
        },
        context: ctx.symbol_context().cloned(),
    })
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
