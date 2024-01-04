mod function;
mod helpers;
mod import;
mod symbol;
mod symbol_table;
mod type_alias;
mod types;

use dossier_core::tree_sitter::{Node, Parser};
use dossier_core::Result;

use symbol::SymbolContext;
use symbol_table::{ScopeID, SymbolTable};

use std::path::Path;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TypeScriptParser {}

impl TypeScriptParser {
    pub fn new() -> Self {
        Self::default()
    }
}

const LANGUAGE: &str = "ts";

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

            let symbol_table = parse_file(ctx)?;

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
            for symbol in table.all_symbols() {
                let entity = symbol.as_entity();
                entities.push(entity);
            }
        }

        Ok(entities)
    }
}

fn parse_file(mut ctx: ParserContext) -> Result<SymbolTable> {
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
                handle_node(&tmp.node(), &mut ctx)?;
            }
            _ => {
                handle_node(&cursor.node(), &mut ctx)?;
            }
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(ctx.take_symbol_table())
}

fn handle_node(node: &Node, ctx: &mut ParserContext) -> Result<()> {
    match node.kind() {
        import::NODE_KIND => {
            let import = import::parse(node, ctx)?;
            ctx.symbol_table.add_import(import);
        }
        function::NODE_KIND => {
            let symbol = function::parse(node, ctx)?;
            ctx.symbol_table
                .add_symbol(symbol.identifier().to_owned().as_str(), symbol);
        }
        type_alias::NODE_KIND => {
            let symbol = type_alias::parse(node, ctx)?;
            ctx.symbol_table
                .add_symbol(symbol.identifier().to_owned().as_str(), symbol);
        }
        _ => {
            println!("Unhandled node: {}", node.kind());
        }
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParserContext<'a> {
    file: &'a Path,
    code: &'a str,
    symbol_table: SymbolTable,
    pub symbol_context: Vec<SymbolContext>,
}

impl<'a> ParserContext<'a> {
    fn new(path: &'a Path, code: &'a str) -> Self {
        Self {
            file: path,
            code,
            symbol_table: SymbolTable::new(path),
            symbol_context: vec![],
        }
    }

    fn take_symbol_table(self) -> SymbolTable {
        self.symbol_table
    }

    fn construct_fqn(&self, identifier: &str) -> String {
        self.symbol_table.construct_fqn(identifier)
    }

    pub fn push_scope(&mut self, name: &str) -> ScopeID {
        self.symbol_table.push_scope(name)
    }

    pub fn pop_scope(&mut self) {
        self.symbol_table.pop_scope();
    }

    pub fn push_context(&mut self, context: SymbolContext) {
        self.symbol_context.push(context);
    }

    pub fn pop_context(&mut self) {
        self.symbol_context.pop();
    }

    pub fn symbol_context(&self) -> Option<&SymbolContext> {
        self.symbol_context.last()
    }
}

#[cfg(test)]
mod test {
    use indoc::indoc;

    use crate::types::Type;

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

        let table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let symbols = table.all_symbols().collect::<Vec<_>>();

        let symbol = symbols[0];
        let function = symbol.kind.as_function().unwrap();

        assert_eq!(function.identifier, "foo".to_string());
        assert_eq!(
            function.documentation,
            Some("The documentation".to_string())
        );
        assert_eq!(function.identifier, "foo".to_string());
        assert_eq!(symbol.fqn, "index.ts::foo");

        let symbol = symbols[1];
        let function = symbol.kind.as_function().unwrap();

        assert_eq!(function.identifier, "bar".to_string());
        assert_eq!(function.documentation, None);
        assert_eq!(
            function.return_type().as_ref().unwrap().kind.as_type(),
            Some(&Type::Predefined("string".to_owned()))
        );

        assert_eq!(symbols.len(), 2);
    }

    #[test]
    fn parses_imports_from_a_file() {
        let source = indoc! { r#"
        import { Foo } from "./foo.ts";

        export function makeFoo(): Foo {
            return new Foo();
        }
        "#};

        let table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1);

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

        let table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1);

        let symbol = symbols[0];
        let alias = symbol.kind.as_type_alias().unwrap();

        assert_eq!(alias.identifier, "Foo");
        assert_eq!(
            alias.the_type().kind.as_type(),
            Some(&Type::Predefined("string".to_owned()))
        );
    }

    #[test]
    fn resolves_type_aliases_in_one_file() {
        let source = indoc! { r#"
        type Foo = string;

        export function makeFoo(): Foo {
            return new Foo();
        }
        "#};

        let mut table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        table.resolve_types();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 2);

        let function = symbols[1].kind.as_function().unwrap();

        assert_eq!(
            function.return_type().as_ref().unwrap().kind.as_type(),
            Some(&Type::Identifier(
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

        let mut foo_table = parse_file(ParserContext::new(Path::new("foo.ts"), foo_file)).unwrap();
        let mut index_table =
            parse_file(ParserContext::new(Path::new("index.ts"), index_file)).unwrap();

        foo_table.resolve_types();
        index_table.resolve_types();

        let all_tables = vec![&foo_table];

        index_table.resolve_imported_types(all_tables);

        let symbols = index_table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1);
        let function = symbols[0].kind.as_function().unwrap();

        assert_eq!(
            function.return_type().as_ref().unwrap().kind.as_type(),
            Some(&Type::Identifier(
                "Foo".to_owned(),
                Some("foo.ts::Foo".to_owned())
            ))
        );
    }
}
