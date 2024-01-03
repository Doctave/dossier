mod function;
mod helpers;
mod import;
mod symbols;
mod type_alias;
mod type_kind;

use dossier_core::tree_sitter::{Node, Parser};
use dossier_core::Result;

use symbols::{SymbolTable, TableEntry};

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeScriptParser {}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self::default()
    }
}

const LANGUAGE: &str = "ts";

trait IntoEntity {
    fn into_entity(self) -> dossier_core::Entity;
}

impl dossier_core::DocsParser for TypeScriptParser {
    fn parse<'a, P: Into<&'a Path>, T: IntoIterator<Item = P>>(
        &self,
        paths: T,
        _ctx: &mut dossier_core::Context,
    ) -> Result<Vec<dossier_core::Entity>> {
        let mut symbols = Vec::new();

        for path in paths {
            let path = path.into();

            let code = std::fs::read_to_string(path).unwrap();
            let ctx = ParserContext::new(path, &code);

            let symbol_table = parse_file(&ctx)?;

            symbols.push(symbol_table);
        }

        for table in symbols.iter_mut() {
            table.resolve_types();
        }

        let mut window = vec![];

        while let Some(mut table) = symbols.pop() {
            table.resolve_imported_types(symbols.iter().chain(window.iter()));
            window.push(table);
        }

        let mut entities = vec![];
        for table in window {
            for entry in table.all_entries() {
                match entry.symbol.kind {
                    symbols::SymbolKind::Function(ref function) => {
                        entities.push(function.clone().into_entity());
                    }
                    _ => {}
                }
            }
        }

        Ok(entities)
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
    fn parse(
        node: &Node,
        table: &mut SymbolTable,
        ctx: &ParserContext,
    ) -> Result<(String, TableEntry)>;
}

fn handle_node(node: &Node, table: &mut SymbolTable, ctx: &ParserContext) -> Result<()> {
    match node.kind() {
        import::NODE_KIND => {
            let import = import::parse(node, table, ctx)?;
            table.add_import(import);
        }
        function::NODE_KIND => {
            let (identifier, entry) = function::parse(node, table, ctx)?;
            table.add_symbol(&identifier, entry);
        }
        type_alias::NODE_KIND => {
            let (identifier, entry) = type_alias::parse(node, table, ctx)?;
            table.add_symbol(&identifier, entry);
        }
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

    use crate::type_kind::TypeKind;

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

        export function bar(): string {
            console.log("Hello, world!");
        }
        "#};

        let table = parse_file(&ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let entries = table.all_entries().collect::<Vec<_>>();

        let entry = entries[0];
        let function = entry.symbol.kind.function().unwrap();

        assert_eq!(function.identifier, "foo".to_string());
        assert_eq!(
            function.documentation,
            Some("The documentation".to_string())
        );

        let entry = entries[1];
        let function = entry.symbol.kind.function().unwrap();

        assert_eq!(function.identifier, "bar".to_string());
        assert_eq!(function.documentation, None);
        assert_eq!(
            function.return_type,
            Some(TypeKind::Predefined("string".to_owned()))
        );

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

    #[test]
    fn parses_type_definitions() {
        let source = indoc! { r#"
        type Foo = string;
        "#};

        let table = parse_file(&ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let entries = table.all_entries().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);

        let entry = entries[0];
        let alias = entry.symbol.kind.type_alias().unwrap();

        assert_eq!(alias.identifier, "Foo");
        assert_eq!(alias.type_kind, TypeKind::Predefined("string".to_owned()));
    }

    #[test]
    fn resolves_type_aliases_in_one_file() {
        let source = indoc! { r#"
        type Foo = string;

        export function makeFoo(): Foo {
            return new Foo();
        }
        "#};

        let mut table = parse_file(&ParserContext::new(Path::new("index.ts"), source)).unwrap();

        table.resolve_types();

        let entries = table.all_entries().collect::<Vec<_>>();
        assert_eq!(entries.len(), 2);

        let function = entries[1].symbol.kind.function().unwrap();

        assert_eq!(
            function.return_type,
            Some(TypeKind::Identifier(
                "Foo".to_owned(),
                Some("index.ts::Foo".to_owned())
            ))
        );
    }

    #[test]
    fn resolves_type_aliases_across_files() {
        let foo_file = indoc! { r#"
        export type Foo = string;
        "#};

        let index_file = indoc! { r#"
        import { Foo } from "./foo.ts";

        export function makeFoo(): Foo {
            return new Foo();
        }
        "#};

        let mut foo_table = parse_file(&ParserContext::new(Path::new("foo.ts"), foo_file)).unwrap();
        let mut index_table =
            parse_file(&ParserContext::new(Path::new("index.ts"), index_file)).unwrap();

        foo_table.resolve_types();
        index_table.resolve_types();

        let all_tables = vec![&foo_table];

        index_table.resolve_imported_types(all_tables);

        let entries = index_table.all_entries().collect::<Vec<_>>();
        assert_eq!(entries.len(), 1);
        let function = entries[0].symbol.kind.function().unwrap();

        assert_eq!(
            function.return_type,
            Some(TypeKind::Identifier(
                "Foo".to_owned(),
                Some("foo.ts::Foo".to_owned())
            ))
        );
    }
}
