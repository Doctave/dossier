mod function;
mod helpers;
mod import;
mod symbols;

use dossier_core::tree_sitter::{Node, Parser};
use dossier_core::Result;

use symbols::{SymbolTable, TableEntry};

use std::path::Path;

pub struct TypeScriptParser {}

impl dossier_core::DocsParser for TypeScriptParser {
    fn parse<'a, P: Into<&'a Path>, T: IntoIterator<Item = P>>(
        &self,
        paths: T,
        _ctx: &mut dossier_core::Context,
    ) -> Result<Vec<dossier_core::Entity>> {
        let mut symbols = vec![];

        for path in paths {
            let path = path.into();

            let code = std::fs::read_to_string(path).unwrap();
            let ctx = ParserContext::new(path, &code);

            symbols.push(parse_file(&ctx)?);
        }

        unimplemented!()
    }
}

fn parse_file(ctx: &ParserContext) -> Result<SymbolTable> {
    let mut table = SymbolTable::new(ctx.path);
    let mut parser = Parser::new();

    parser
        .set_language(tree_sitter_typescript::language_typescript())
        .expect("Error loading TypeScript grammar");

    let tree = parser.parse(ctx.code, None).unwrap();

    let mut cursor = tree.root_node().walk();
    assert_eq!(cursor.node().kind(), "program");
    cursor.goto_first_child();

    loop {
        println!(">> {}", cursor.node().kind());

        match cursor.node().kind() {
            "comment" => {
                // Skip comments
            }
            "export_statement" => {
                let mut tmp = cursor.node().walk();
                tmp.goto_first_child();
                tmp.goto_next_sibling();
                handle_node(&tmp.node(), &mut table, ctx)?;
            }
            _ => {
                handle_node(&cursor.node(), &mut table, ctx)?;
            }
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(table)
}

pub(crate) trait ParseSymbol {
    /// A trait for parsing a symbol into a table entry from a node.
    ///
    /// Can be called recursive to construct 
    fn parse(node: &Node, table: &mut SymbolTable, ctx: &ParserContext) -> Result<(String, TableEntry)>;
}

fn handle_node(node: &Node, table: &mut SymbolTable, ctx: &ParserContext) -> Result<()> {
    println!("Handling node {}", node.kind());
    match node.kind() {
        import::NODE_KIND => {
            let import = import::parse(node, table, ctx)?;
            table.add_import(import);
        },
        function::NODE_KIND => {
            let (identifier, entry) = function::parse(node, table, ctx)?;
            table.add_symbol(&identifier, entry);
        },
        _ => {
            println!("Unhandled node: {}", node.kind());
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParserContext<'a> {
    path: &'a Path,
    code: &'a str,
}

impl<'a> ParserContext<'a> {
    fn new(path: &'a Path, code: &'a str) -> Self {
        Self { path, code }
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use super::*;

    #[test]
    fn parses_a_file_with_functions() {
        let source = indoc! { r#"
        /**
         * The documentation
         */
        export function foo() {
            console.log("Hello, world!");
        }

        export function bar() {
            console.log("Hello, world!");
        }
        "#};

        let table = parse_file(&ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let entries = table.all_entries().collect::<Vec<_>>();

        // assert_eq!(
        //     entries[0],
        //     &TableEntry::Symbol(Symbol {
        //         kind: SymbolKind::Function(Function {
        //             title: "foo".to_string(),
        //             documentation: Some("The documentation".to_string()),
        //             is_exported: true,
        //         }),
        //         source: Source {
        //             offset_start_bytes: 36,
        //             offset_end_bytes: 88,
        //         },
        //     })
        // );

        // assert_eq!(
        //     entries[1],
        //     &TableEntry::Symbol(Symbol {
        //         kind: SymbolKind::Function(Function {
        //             title: "bar".to_string(),
        //             documentation: None,
        //             is_exported: true,
        //         }),
        //         source: Source {
        //             offset_start_bytes: 97,
        //             offset_end_bytes: 149,
        //         },
        //     })
        // );
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn parses_imports_from_a_file() {
        let source = indoc! { r#"
        import { Foo } from "./foo.ts";

        export function makeFoo(): Foo {
            return new Foo();
        }
        "#};

        let table = parse_file(&ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let entries = table.all_entries().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);

        let imports = table.all_imports().collect::<Vec<_>>();
        assert_eq!(imports.len(), 1);

        assert_eq!(imports[0].names, vec!["Foo"]);
        assert_eq!(imports[0].source, "./foo.ts");
    }
}
