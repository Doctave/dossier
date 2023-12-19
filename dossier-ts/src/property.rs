use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Config, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

const QUERY_STRING: &str = indoc! {"
      (property_signature 
         name: (property_identifier) @property_name
         type: (type_annotation) @property_type
      ) @property
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
            let main_node = node_for_capture("property", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("property_name", m.captures, &QUERY).unwrap();
            let mut type_node = node_for_capture("property_type", m.captures, &QUERY).unwrap();
            // Go to the node that actually contains the whole type
            let mut cursor = type_node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            type_node = cursor.node();

            let interface_docs = find_docs(main_node, code).map(crate::process_comment);

            Entity {
                title: name_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                description: interface_docs.unwrap_or("".to_owned()),
                kind: "property".to_string(),
                children: vec![],
                language: "ts".to_owned(),
                meta: json!({
                    "type".to_owned():
                    type_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                }),
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

fn find_docs<'a>(node: Node<'a>, code: &'a str) -> Option<&'a str> {
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

#[cfg(test)]
mod test {
    /// NOTE: Properties are always inside some kind of context, so tests
    /// need to create a parent node context in order to parse them correctly.
    use super::*;
    use indoc::indoc;

    // fn parent_node<'a>(code: &'a str) -> Node<'a> {}

    #[test]
    fn parses_properties() {
        let code = indoc! { r#"
        interface ExampleInterface {
            label: string;
            optional?: string;
            readonly age: number;
        }
        "#};

        let mut parser = dossier_core::tree_sitter::Parser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading Rust grammar");

        let tree = parser.parse(code.clone(), None).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&crate::interface::QUERY, tree.root_node(), code.as_bytes());

        let parent_captures = matches
            .into_iter()
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .captures;

        let root =
            node_for_capture("interface_body", parent_captures, &crate::interface::QUERY).unwrap();

        let properties = parse_from_node(root, Path::new("index.ts"), code, &Config {}).unwrap();

        assert_eq!(properties.len(), 3);

        let mut property = &properties[0];
        assert_eq!(property.title, "label");
        assert_eq!(property.kind, "property");
        assert_eq!(property.meta, json!({ "type": "string" }),);

        property = &properties[1];
        assert_eq!(property.title, "optional");
        assert_eq!(property.kind, "property");
        assert_eq!(property.meta, json!({ "type": "string", "optional": true }),);

        property = &properties[2];
        assert_eq!(property.title, "label");
        assert_eq!(property.kind, "property");
        assert_eq!(property.meta, json!({ "type": "number", "readonly": true }),);
    }
}
