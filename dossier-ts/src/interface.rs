use dossier_core::{helpers::*, Config, Entity, Result, Source};
use lazy_static::lazy_static;
use tree_sitter::{Node, Parser, Query, QueryCursor};

use std::path::Path;

const INTERFACE_DECLARATION_QUERY: &str = "
      (interface_declaration
         name: (type_identifier) @interface_name
         type_parameters: (type_parameters) ? @type_parameters
      )
    ";

lazy_static! {
    static ref QUERY: Query = Query::new(
        tree_sitter_typescript::language_typescript(),
        &format!(
            r#"
        [
            (
               (comment) ? @comment
               (export_statement 
                  {}
                )
             )
            (
               (comment) ? @comment
               {}
             )
         ]
        "#,
            INTERFACE_DECLARATION_QUERY, INTERFACE_DECLARATION_QUERY,
        )
    )
    .unwrap();
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

    let comment_index = QUERY.capture_index_for_name("comment").unwrap();
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

            let comment = get_string_from_match(m.captures, comment_index, code)
                .map(|t| t.unwrap())
                .unwrap_or("");

            let comment = crate::process_comment(comment);

            Entity {
                title: format!("{}{}", interface_name, type_parameters),
                description: comment.to_owned(),
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
        let interface = &result[0];

        assert_eq!(interface.title, "TheExportedInterface");
        assert_eq!(interface.kind, "interface");
        assert_eq!(interface.language, "ts");
        assert_eq!(
            interface.description,
            "This is the comment\n\nWith more lines"
        );
    }
}
