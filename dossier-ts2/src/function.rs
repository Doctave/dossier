use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Result};
use indoc::indoc;
use lazy_static::lazy_static;

use crate::helpers::*;
use crate::symbols::Source;
use crate::{
    symbols::{Symbol, SymbolKind, SymbolTable, TableEntry},
    ParserContext,
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
    pub title: String,
    pub documentation: Option<String>,
    pub is_exported: bool,
}

pub(crate) fn parse(
    node: &Node,
    table: &mut SymbolTable,
    ctx: &ParserContext,
) -> Result<(String, Symbol)> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = QueryCursor::new();
    let function = cursor
        .matches(&QUERY, *node, ctx.code.as_bytes())
        .into_iter()
        .next()
        .unwrap();

    let main_node = node_for_capture("function", function.captures, &QUERY).unwrap();
    let name_node = node_for_capture("function_name", function.captures, &QUERY).unwrap();
    // let parameter_node = node_for_capture("function_parameters", m.captures, &QUERY);
    // let return_type = node_for_capture("function_return_type", m.captures, &QUERY);

    let docs = find_docs(&main_node, ctx.code);

    let title = name_node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();

    Ok((
        title.clone(),
        Symbol {
            kind: SymbolKind::Function(Function {
                title,
                documentation: docs.map(process_comment),
                is_exported: is_exported(&main_node),
            }),
            source: Source {
                offset_start_bytes: main_node.start_byte(),
                offset_end_bytes: main_node.end_byte(),
            },
        },
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
