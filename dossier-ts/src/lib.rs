use dossier_core::{Config, DocsParser, Entity, Result, Source};
use tree_sitter::{Parser as TParser, Query, QueryCursor};

use std::path::Path;

mod interface;

pub struct Parser {}

impl DocsParser for Parser {
    fn parse(&self, path: &Path, _config: &Config) -> Result<Vec<Entity>> {
        let code = std::fs::read_to_string(path).unwrap();

        let mut parser = TParser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading Rust grammar");

        let tree = parser.parse(code.clone(), None).unwrap();

        let query = Query::new(
            tree_sitter_typescript::language_typescript(),
            r#"
             (
                (comment) @comment
                (export_statement 
                   (interface_declaration
                      name: (type_identifier) @interface_name
                      type_parameters: (type_parameters) @type_parameters
                   )
                 )
              )
             "#,
        )
        .unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), code.as_bytes());

        let comment_index = query.capture_index_for_name("comment").unwrap();
        let interface_name_index = query.capture_index_for_name("interface_name").unwrap();
        let type_parameters_index = query.capture_index_for_name("type_parameters").unwrap();

        Ok(matches
            .into_iter()
            .map(|m| {
                let interface_name = m
                    .captures
                    .iter()
                    .find(|c| c.index == interface_name_index)
                    .unwrap()
                    .node
                    .utf8_text(code.as_bytes())
                    .unwrap();

                let type_parameters = m
                    .captures
                    .iter()
                    .find(|c| c.index == type_parameters_index)
                    .unwrap()
                    .node
                    .utf8_text(code.as_bytes())
                    .unwrap();

                let comment = m
                    .captures
                    .iter()
                    .find(|c| c.index == comment_index)
                    .unwrap()
                    .node
                    .utf8_text(code.as_bytes())
                    .unwrap();

                let comment = process_comment(comment);

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
}

pub(crate) fn process_comment(comment: &str) -> String {
    let mut tmp = comment.trim().to_owned();
    tmp = tmp.trim_start_matches("/**").to_owned();
    tmp = tmp.trim_end_matches("*/").to_owned();

    tmp.lines()
        .map(|l| l.trim().trim_start_matches("* ").trim_start_matches('*'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}
