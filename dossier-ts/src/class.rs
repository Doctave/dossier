use dossier_core::tree_sitter::{Node, Parser, Query, QueryCursor};
use dossier_core::{helpers::*, serde_json::json, Context, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

use crate::{field, method};

const QUERY_STRING: &str = indoc! {"
      [
        (
          class_declaration 
              name: (type_identifier) @class_name
              body: (class_body) @class_body
        ) @class
        (
          abstract_class_declaration 
              name: (type_identifier) @class_name
              body: (class_body) @class_body
        ) @class
      ]
    "};

lazy_static! {
    pub(crate) static ref QUERY: Query =
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
            let main_node = node_for_capture("class", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("class_name", m.captures, &QUERY).unwrap();
            let body_node = node_for_capture("class_body", m.captures, &QUERY).unwrap();

            let docs = find_docs(main_node, code).map(crate::process_comment);

            let mut members = vec![];

            members.append(&mut field::parse_from_node(body_node, path, code, ctx).unwrap());
            members.append(&mut method::parse_from_node(body_node, path, code, ctx).unwrap());

            Entity {
                title: name_node.utf8_text(code.as_bytes()).unwrap().to_owned(),
                description: docs.unwrap_or(String::new()),
                kind: "class".to_string(),
                fqn: "TODO".to_string(),
                language: "ts".to_owned(),
                meta: json!({}),
                members,
                member_context: None,
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
    use super::*;

    #[test]
    fn parses_class() {
        let code = indoc! { r#"
        /**
         * Greeter class
         */
        class Greeter {
          greeting: string;
         
          constructor(message: string) {
            this.greeting = message;
          }
         
          greet() {
            return "Hello, " + this.greeting;
          }
        }
        "#};

        let result = parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let class = &result[0];

        assert_eq!(class.title, "Greeter");
        assert_eq!(class.description, "Greeter class");

        // --- Constructor ------------------

        let constructor = class
            .members
            .iter()
            .find(|m| m.title == "constructor")
            .unwrap();

        assert_eq!(constructor.kind, "method");

        let properties = constructor
            .members
            .iter()
            .filter(|m| m.kind == "parameter")
            .collect::<Vec<_>>();

        assert_eq!(properties.len(), 1);
        assert_eq!(properties[0].title, "message");
        assert_eq!(properties[0].kind, "parameter");
        assert_eq!(properties[0].members[0].title, "string");
        assert_eq!(properties[0].members[0].kind, "type");

        // --- Method -----------------------

        let method = class.members.iter().find(|m| m.title == "greet").unwrap();

        assert_eq!(method.kind, "method");

        // --- Fields -----------------------

        let fields = class
            .members
            .iter()
            .filter(|m| m.kind == "field")
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);

        let field = &fields[0];
        assert_eq!(field.title, "greeting");
        assert_eq!(field.members[0].title, "string");
        assert_eq!(field.members[0].kind, "type");
    }
}
