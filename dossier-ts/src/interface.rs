use dossier_core::tree_sitter::{Node, Parser, Query, QueryCursor};
use dossier_core::{helpers::*, Config, Entity, Result, Source};
use lazy_static::lazy_static;

use std::path::Path;

const INTERFACE_DECLARATION_QUERY: &str = "
      (interface_declaration
         name: (type_identifier) @interface_name
         type_parameters: (type_parameters) ? @type_parameters
      ) @interface_body
    ";

lazy_static! {
    static ref QUERY_STRING: String = format!(
        r#"
         [
            ( (comment)* @interface_docs . (export_statement . {} ))
            ( (comment)* @interface_docs . {})
         ]
        "#,
        INTERFACE_DECLARATION_QUERY, INTERFACE_DECLARATION_QUERY,
    );
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), &QUERY_STRING).unwrap();
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

    let interface_docs_index = QUERY.capture_index_for_name("interface_docs").unwrap();
    let interface_name_index = QUERY.capture_index_for_name("interface_name").unwrap();
    let type_parameters_index = QUERY.capture_index_for_name("type_parameters").unwrap();

    Ok(matches
        .into_iter()
        .map(|m| {
            let interface_name = get_string_from_match(m.captures, interface_name_index, code)
                .unwrap()
                .unwrap();

            let type_parameters = get_string_from_match(m.captures, type_parameters_index, code)
                .map(|t| t.unwrap())
                .unwrap_or("");

            let interface_docs = get_string_from_match(m.captures, interface_docs_index, code)
                .map(|t| t.unwrap())
                .unwrap_or("");

            let interface_docs = crate::process_comment(interface_docs);

            Entity {
                title: format!("{}{}", interface_name, type_parameters),
                description: interface_docs,
                kind: "interface".to_string(),
                children: vec![],
                language: "ts".to_owned(),
                source: Source {
                    file: path.to_owned(),
                    start_offset_bytes: 0,
                    end_offset_bytes: 0,
                    repository: None,
                },
            }
        })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parse_interface_name() {
        let code = indoc! { r#"
        interface TheInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "TheInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "");
    }

    #[test]
    fn parse_exported_interface_name() {
        let code = indoc! { r#"
        export interface TheExportedInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "");
    }

    #[test]
    fn parse_interface_docstring() {
        let code = indoc! { r#"
        /**
         * This is the comment
         */
        interface TheExportedInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is the comment");
    }

    #[test]
    fn parse_exported_interface_docstring() {
        let code = indoc! { r#"
        /**
         * This is the comment
         */
        export interface TheExportedInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
        println!("{:#?}", result);
        assert_eq!(result.len(), 1);
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is the comment");
    }

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

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
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
    fn parse_multiple_interfaces() {
        let code = indoc! { r#"
        /**
         * This is the comment
         */
        interface TheExportedInterface {}

        /**
         * This is another comment
         */
        interface TheOtherExportedInterface {}
        "#};

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
        assert_eq!(result.len(), 2);
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is the comment");

        let interface = &result[1];

        assert_eq!(interface.title, "TheOtherExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(interface.description, "This is another comment");
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

        let result = parse(code, Path::new("index.ts"), &Config {}).expect("Failed to parse code");
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
}
