use dossier_core::tree_sitter::{Node, Parser, Query, QueryCursor};
use dossier_core::Identity;
use dossier_core::{helpers::*, serde_json::json, Context, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

use crate::parameter;

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

pub(crate) fn parse(code: &str, path: &Path, ctx: &mut Context) -> Result<Vec<Entity>> {
    let mut parser = Parser::new();

    parser
        .set_language(tree_sitter_typescript::language_typescript())
        .expect("Error loading TypeScript grammar");

    let tree = parser.parse(code.clone(), None).unwrap();

    parse_from_node(tree.root_node(), path, code, ctx)
}

pub(crate) fn parse_from_node(
    node: Node,
    path: &Path,
    code: &str,
    ctx: &mut Context,
) -> Result<Vec<Entity>> {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&QUERY, node, code.as_bytes());

    Ok(matches
        .into_iter()
        .map(|m| {
            let main_node = node_for_capture("function", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("function_name", m.captures, &QUERY).unwrap();
            let parameter_node = node_for_capture("function_parameters", m.captures, &QUERY);
            let return_type = node_for_capture("function_return_type", m.captures, &QUERY);

            let docs = find_docs(&main_node, code);

            let meta = json!({});
            let mut members = vec![];

            if let Some(return_type) = return_type {
                members.push(parse_return_type(&return_type, path, code, ctx));
            }

            if let Some(parameters) = parameter_node {
                members.append(
                    &mut parameter::parse_from_node(&parameters, path, code, &Context::new())
                        .unwrap(),
                );
            }

            Entity {
                title: name_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                description: docs.map(|s| s.to_owned()).unwrap_or("".to_string()),
                kind: "function".to_string(),
                identity: Identity::FQN("TODO".to_string()),
                members,
                member_context: None,
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

fn parse_return_type<'a>(node: &Node<'a>, path: &Path, code: &'a str, _ctx: &mut Context) -> Entity {
    let title = node
        .utf8_text(code.as_bytes())
        .unwrap()
        .trim_start_matches(": ")
        .to_owned();

    Entity {
        title,
        description: "".to_string(),
        kind: "type".to_string(),
        identity: Identity::FQN("TODO".to_string()),
        members: vec![],
        member_context: Some("returnType".to_string()),
        language: "ts".to_owned(),
        meta: json!({}),
        source: Source {
            file: path.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
            repository: None,
        },
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

#[cfg(test)]
mod test {
    use super::*;
    use dossier_core::serde_json::Value;
    use indoc::indoc;

    #[test]
    fn parses_function() {
        let code = indoc! { r#"
        /**
         * This is the comment
         */
        function example(foo: string, bar?: number): boolean {
            return true
        }
        "#};

        let result =
            parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);

        let function = &result[0];
        assert_eq!(function.title, "example");
        assert_eq!(function.kind, "function");

        let return_type = &function
            .members
            .iter()
            .find(|m| m.member_context == Some("returnType".to_string()))
            .unwrap();
        assert_eq!(return_type.title, "boolean");
        assert_eq!(return_type.kind, "type");

        let parameters = &function
            .members
            .iter()
            .filter(|m| m.member_context == Some("parameter".to_string()))
            .collect::<Vec<_>>();
        assert_eq!(parameters.len(), 2);

        let foo = &parameters[0];
        assert_eq!(foo.title, "foo");
        assert_eq!(foo.kind, "parameter");
        assert_eq!(foo.members[0].title, "string");
        assert_eq!(foo.members[0].kind, "type");

        let bar = &parameters[1];
        assert_eq!(bar.title, "bar");
        assert_eq!(bar.kind, "parameter");
        assert_eq!(bar.meta["optional"], Value::Bool(true));
        assert_eq!(bar.members[0].title, "number");
        assert_eq!(bar.members[0].kind, "type");
    }
}
