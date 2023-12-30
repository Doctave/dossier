use dossier_core::tree_sitter::{Node, Parser, Query, QueryCursor};
use dossier_core::{helpers::*, serde_json::json, Context, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

use crate::{method, property};

const QUERY_STRING: &str = indoc! {"
      (interface_declaration
         name: (type_identifier) @interface_name
         type_parameters: (type_parameters) ? @type_parameters
      ) @interface_body
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

    let interface_name_index = QUERY.capture_index_for_name("interface_name").unwrap();
    let type_parameters_index = QUERY.capture_index_for_name("type_parameters").unwrap();

    Ok(matches
        .into_iter()
        .map(|m| {
            let main_node = node_for_capture("interface_body", m.captures, &QUERY).unwrap();
            let interface_name = get_string_from_match(m.captures, interface_name_index, code)
                .unwrap()
                .unwrap();

            let type_parameters = get_string_from_match(m.captures, type_parameters_index, code)
                .map(|t| t.unwrap())
                .unwrap_or("");

            let interface_docs = find_docs(main_node, code).map(crate::process_comment);

            let mut members = vec![];

            ctx.push_namespace(interface_name);

            members.append(&mut property::parse_from_node(main_node, path, code, ctx).unwrap());
            members.append(&mut method::parse_from_node(main_node, path, code, ctx).unwrap());

            ctx.pop_namespace();

            Entity {
                title: format!("{}{}", interface_name, type_parameters),
                description: interface_docs.unwrap_or("".to_owned()),
                kind: "interface".to_string(),
                fqn: ctx.generate_fqn(path, [interface_name]),
                members,
                member_context: Some("interface".to_string()),
                language: "ts".to_owned(),
                meta: json!({}),
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
    fn parse_multiline_interface_docstring() {
        let code = indoc! { r#"
        /**
         * This is the comment
         *
         * With more lines
         */
        interface TheExportedInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(
            interface.description,
            "This is the comment\n\nWith more lines"
        );
    }

    #[test]
    fn parse_multiple_interfaces_with_without_comments_and_export() {
        let code = indoc! { r#"
        /**
         * This is the comment
         */
        interface Interface1 {}

        /**
         * This is another comment
         */
        export interface Interface2 {}

        interface Interface3 {}

        export interface Interface4 {}
        "#};

        let result = parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        println!("{:#?}", result);
        assert_eq!(result.len(), 4);
        let interface = &result[0];

        assert_eq!(interface.title, "Interface1");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is the comment");

        let interface = &result[1];

        assert_eq!(interface.title, "Interface2");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is another comment");

        let interface = &result[2];

        assert_eq!(interface.title, "Interface3");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "");

        let interface = &result[3];

        assert_eq!(interface.title, "Interface4");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "");
    }

    #[test]
    fn parse_interface_with_properties() {
        let code = indoc! { r#"
        interface ExampleInterface {
            label: string,
            age: intger
        }
        "#};

        let result = parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "ExampleInterface");
        assert_eq!(interface.kind, "interface");

        let property = &interface.members[0];
        assert_eq!(property.title, "label");
        assert_eq!(property.kind, "property");
        assert_eq!(property.member_context.as_deref(), Some("property"));

        let property_type = &property.members[0];

        assert_eq!(property_type.title, "string");
    }

    #[test]
    fn parse_interface_with_methods() {
        let code = indoc! { r#"
        interface ExampleInterface {
          /**
           * Creates the OperationNode that describes how to compile this expression into SQL.
           */
          toOperationNode(): AliasNode
        }
        "#};

        let result = parse(code, Path::new("index.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "ExampleInterface");
        assert_eq!(interface.kind, "interface");

        let property = &interface.members[0];
        assert_eq!(property.title, "toOperationNode");
        assert_eq!(property.kind, "method");
        assert_eq!(property.member_context.as_deref(), Some("method"));

        assert_eq!(property.members.len(), 1);
        let return_type = property.members.first().unwrap();

        assert_eq!(return_type.member_context.as_deref(), Some("returnType"));
        assert_eq!(return_type.kind, "type");
        assert_eq!(return_type.title, "AliasNode");
    }

    #[test]
    fn computes_fqns() {
        let code = indoc! { r#"
        interface ExampleInterface {
          label: string,

          exampleMethod(): ExampleType
        }
        "#};

        let result = parse(code, Path::new("src/example.ts"), &mut Context::new()).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.fqn, "src/example.ts::ExampleInterface");

        assert_eq!(interface.members[0].fqn, "src/example.ts::ExampleInterface::label");
        assert_eq!(interface.members[1].fqn, "src/example.ts::ExampleInterface::exampleMethod");
    }
}
