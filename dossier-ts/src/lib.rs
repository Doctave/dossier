use dossier_core::Result;
use tree_sitter::{Parser as TParser, Query, QueryCursor};

use std::path::Path;

pub struct Parser {}

impl Parser {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<()> {
        let code = std::fs::read_to_string(path).unwrap();

        let mut parser = TParser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading Rust grammar");

        let tree = parser.parse(code.clone(), None).unwrap();

        let query = Query::new(
            tree_sitter_typescript::language_typescript(),
            "((comment)* @class_doc .(export_statement (class_declaration name: (type_identifier) @name body: (class_body) @body )))",
        ).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&query, tree.root_node(), code.as_bytes());

        let class_doc_index = query.capture_index_for_name("class_doc").unwrap();
        let class_name_index = query.capture_index_for_name("name").unwrap();

        println!("Classes:");

        for m in matches {
            println!("docs: {:?}", m.captures.iter().find(|c| c.index == class_doc_index).unwrap().node.utf8_text(code.as_bytes()).unwrap());
            println!("class: {:?}", m.captures.iter().find(|c| c.index == class_name_index).unwrap().node.utf8_text(code.as_bytes()).unwrap());
        }

        Ok(())
    }
}
