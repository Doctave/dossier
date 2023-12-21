use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Config, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

const QUERY_STRING: &str = indoc! {"
    (method_signature 
        name: (property_identifier) @method_name
        parameters: (formal_parameters) @method_parameters
        return_type: (type_annotation) ? @method_return_type
    ) @method
    "};

lazy_static! {
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

pub(crate) fn parse_from_node(
    node: Node,
    path: &Path,
    code: &str,
    _config: &Config,
) -> Result<Vec<Entity>> {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&QUERY, node, code.as_bytes());

    Ok(matches
        .into_iter()
        .map(|m| {
            let main_node = node_for_capture("method", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("method_name", m.captures, &QUERY).unwrap();
            // let parameters_name =
            node_for_capture("method_parameters", m.captures, &QUERY).unwrap();
            let return_type = node_for_capture("method_return_type", m.captures, &QUERY);

            let docs = find_docs(&main_node, code);

            let mut meta = json!({});

            if let Some(return_type) = return_type {
                meta["return_type"] = return_type
                    .utf8_text(code.as_bytes())
                    .unwrap()
                    .trim_start_matches(": ")
                    .into();
            }

            Entity {
                title: name_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                description: docs,
                kind: "method".to_string(),
                children: vec![],
                language: "ts".to_owned(),
                meta,
                source: Source {
                    file: path.to_owned(),
                    start_offset_bytes: main_node.start_byte(),
                    end_offset_bytes: main_node.end_byte(),
                    repository: None,
                },
            }
        })
        .collect::<Vec<_>>())
}

fn find_docs(node: &Node, code: &str) -> String {
    if let Some(previous) = node.prev_sibling() {
        if previous.kind() == "comment" {
            return previous.utf8_text(code.as_bytes()).unwrap().to_owned();
        }
    }

    String::new()
}
