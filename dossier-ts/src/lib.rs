use dossier_core::Result;
use tree_sitter::Parser as TParser;

use std::collections::VecDeque;
use std::path::Path;

pub struct Parser {}

impl Parser {
    pub fn parse<P: AsRef<Path>>(path: P) -> Result<()> {
        let code = std::fs::read_to_string(path).unwrap();

        let mut parser = TParser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading Rust grammar");

        let parsed = parser.parse(code.clone(), None).unwrap();
        let mut cursor = parsed.walk();

        let mut level: usize = 0;

        println!("{}", code);

        println!("------------------------------------------");

        let mut reached_root = false;

        while !reached_root {
            if cursor.node().is_named() {
                println!(
                    "{}{} | {:?}",
                    "  ".repeat(level),
                    cursor.node().kind(),
                    &cursor.node().utf8_text(code.as_bytes()).unwrap()
                );
            }

            if cursor.goto_first_child() {
                level += 1;
                continue;
            }

            if cursor.goto_next_sibling() {
                continue;
            }

            let mut retracing = true;
            while retracing {
                if !cursor.goto_parent() {
                    retracing = false;
                    reached_root = true;
                }

                if cursor.goto_next_sibling() {
                    level -= 1;
                    retracing = false;
                }
            }
        }

        Ok(())
    }
}
