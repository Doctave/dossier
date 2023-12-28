use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Parser, Query, QueryCursor};
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

pub(crate) fn parse(code: &str, path: &Path, config: &Config) -> Result<Vec<Entity>> {
    let mut parser = Parser::new();

    parser
        .set_language(tree_sitter_typescript::language_typescript())
        .expect("Error loading Rust grammar");

    let tree = parser.parse(code.clone(), None).unwrap();

    parse_from_node(tree.root_node(), path, code, config)
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

            let meta = json!({});
            let mut members = vec![];

            if let Some(return_type) = return_type {
                let title = return_type
                    .utf8_text(code.as_bytes())
                    .unwrap()
                    .trim_start_matches(": ")
                    .to_owned();

                members.push(Entity {
                    title,
                    description: "".to_string(),
                    kind: "type".to_string(),
                    members: vec![],
                    member_kind: Some("returnType".to_string()),
                    language: "ts".to_owned(),
                    meta: json!({}),
                    source: Source {
                        file: path.to_owned(),
                        start_offset_bytes: return_type.start_byte(),
                        end_offset_bytes: return_type.end_byte(),
                        repository: None,
                    },
                });
            }

            Entity {
                title: name_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                description: docs,
                kind: "method".to_string(),
                members,
                member_kind: Some("method".to_string()),
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

#[cfg(test)]
mod test {
    use indoc::formatdoc;

    use super::*;

    fn nodes_in_interface_context(code: &str) -> Result<Vec<Entity>> {
        let context = formatdoc! {"
            interface PlaceholderContext {{
                {}
            }}",
            code
        };

        let mut parser = dossier_core::tree_sitter::Parser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading typescript grammar");

        let tree = parser.parse(context.clone(), None).unwrap();

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

        parse_from_node(root, Path::new("index.ts"), &context, &Config {})
    }

    #[test]
    fn method_with_return_type_parameter() {
        let methods = nodes_in_interface_context(indoc! {r#"
            getExpression(): string
        "#})
        .unwrap();

        assert_eq!(methods.len(), 1);

        let method = &methods[0];
        assert_eq!(method.title, "getExpression");
        assert_eq!(method.kind, "method");

        let members = &method.members;
        assert_eq!(members.len(), 1);
        let member = members.first().unwrap();

        assert_eq!(member.member_kind.as_deref(), Some("returnType"));
        assert_eq!(member.kind, "type");
        assert_eq!(member.title, "string");
    }
}