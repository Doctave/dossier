mod class;
mod export_clause;
mod field;
mod function;
mod helpers;
mod import;
mod interface;
mod method;
mod parameter;
mod property;
mod symbol;
mod symbol_table;
mod type_alias;
mod type_constraint;
mod type_variable;
mod types;

use dossier_core::tree_sitter::{Node, Parser};
use dossier_core::Result;

use rayon::prelude::*;

use symbol::SymbolContext;
use symbol_table::{ScopeID, SymbolTable};

use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
        let out = Mutex::new(Vec::new());

        let paths: Vec<PathBuf> = paths
            .into_iter()
            .map(|p| p.into().to_owned())
            .collect::<Vec<_>>();

        paths.as_slice().par_iter().for_each(|path| {
            let code = std::fs::read_to_string(path).unwrap();
            let ctx = ParserContext::new(path, &code);

            // TODO(Nik): Handle error
            let symbol_table = parse_file(ctx).unwrap();

            out.lock().unwrap().push(symbol_table);
        });

        let mut symbols = out.into_inner().unwrap();

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
        class::NODE_KIND => {
            let symbol = class::parse(node, ctx)?;
            ctx.symbol_table.add_symbol(symbol);
        }
        class::ABSTRACT_NODE_KIND => {
            let symbol = class::parse(node, ctx)?;
            ctx.symbol_table.add_symbol(symbol);
        }
        function::NODE_KIND => {
            let symbol = function::parse(node, ctx)?;
            ctx.symbol_table.add_symbol(symbol);
        }
        type_alias::NODE_KIND => {
            let symbol = type_alias::parse(node, ctx)?;
            ctx.symbol_table.add_symbol(symbol);
        }
        interface::NODE_KIND => {
            let symbol = interface::parse(node, ctx)?;
            ctx.symbol_table.add_symbol(symbol);
        }
        export_clause::NODE_KIND => {
            let exported_identifiers = export_clause::parse_exports(node, ctx)?;

            for identifier in exported_identifiers {
                ctx.symbol_table.export_symbol(&identifier);
            }
        }
        _ => {
            // println!("Unhandled node: {}", node.kind());
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

    pub fn push_fqn(&mut self, part: &str) {
        self.symbol_table.push_fqn(part)
    }

    pub fn pop_fqn(&mut self) -> Option<String> {
        self.symbol_table.pop_fqn()
    }

    pub fn push_scope(&mut self) -> ScopeID {
        self.symbol_table.push_scope()
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

    pub fn current_scope(&self) -> ScopeID {
        self.symbol_table.current_scope().id
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
        assert_eq!(symbol.fqn.as_ref().unwrap(), "index.ts::foo");

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
    fn parses_interface() {
        let source = indoc! { r#"
        interface Human {
            name: string;
            age?: number;
        }
        "#};

        let table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1);

        let symbol = symbols[0];
        let interface = symbol.kind.as_interface().unwrap();

        assert_eq!(interface.identifier, "Human");
        let properties = interface.properties().collect::<Vec<_>>();
        assert_eq!(properties.len(), 2);

        assert_eq!(properties[0].identifier().unwrap(), "name");
        assert!(!properties[0].kind.as_property().unwrap().optional);
        let prop_type = properties[0]
            .kind
            .as_property()
            .unwrap()
            .the_type()
            .unwrap();
        assert_eq!(
            prop_type.kind.as_type().unwrap(),
            &Type::Predefined("string".to_owned())
        );

        assert_eq!(properties[1].identifier().unwrap(), "age");
        assert!(properties[1].kind.as_property().unwrap().optional);
        let prop_type = properties[1]
            .kind
            .as_property()
            .unwrap()
            .the_type()
            .unwrap();
        assert_eq!(
            prop_type.kind.as_type().unwrap(),
            &Type::Predefined("number".to_owned())
        );
    }

    #[test]
    fn parses_class() {
        let source = indoc! { r#"
        class Point {
          x = 10;
          y = 10;
         
          scale(n: number): void {
            this.x *= n;
            this.y *= n;
          }
        }
        "#};

        let table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1, "No symbols found");

        let symbol = symbols[0];
        let interface = symbol.kind.as_class().unwrap();

        assert_eq!(interface.identifier, "Point");

        let methods = interface.methods();
        assert_eq!(methods.count(), 1);

        let fields = interface.fields();
        assert_eq!(fields.count(), 2);
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
    fn resolves_type_aliases_in_nested_symbols_in_one_file() {
        let source = indoc! { r#"
        type Foo = string;

        type Bar = {
            foo: Foo;
        }
        "#};

        let mut table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        table.resolve_types();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 2);

        match symbols[1]
            .kind
            .as_type_alias()
            .unwrap()
            .the_type()
            .kind
            .as_type()
            .unwrap()
        {
            Type::Object { properties, .. } => {
                let resolved_type = properties[0].kind.as_property().unwrap().children[0]
                    .kind
                    .as_type()
                    .unwrap();

                assert_eq!(
                    resolved_type,
                    &Type::Identifier("Foo".to_owned(), Some("index.ts::Foo".to_owned()))
                );
            }
            _ => panic!("Expected an object type"),
        }
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

    #[test]
    fn resolves_type_aliases_in_nested_symbols_across_files() {
        let foo_file = indoc! { r#"
        export type Foo = string;
        "#};

        let index_file = indoc! { r#"
        import { Foo } from "./foo.ts";

        type Bar = {
            foo: Foo;
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

        match symbols[0]
            .kind
            .as_type_alias()
            .unwrap()
            .the_type()
            .kind
            .as_type()
            .unwrap()
        {
            Type::Object { properties, .. } => {
                let resolved_type = properties[0].kind.as_property().unwrap().children[0]
                    .kind
                    .as_type()
                    .unwrap();

                assert_eq!(
                    resolved_type,
                    &Type::Identifier("Foo".to_owned(), Some("foo.ts::Foo".to_owned()))
                );
            }
            _ => panic!("Expected an object type"),
        }
    }

    #[test]
    fn does_not_resolves_type_aliases_in_nested_symbols_across_files_if_the_referenced_type_is_not_exported(
    ) {
        let foo_file = indoc! { r#"
        type Foo = string;
        "#};

        let index_file = indoc! { r#"
        import { Foo } from "./foo.ts";

        type Bar = {
            foo: Foo;
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

        match symbols[0]
            .kind
            .as_type_alias()
            .unwrap()
            .the_type()
            .kind
            .as_type()
            .unwrap()
        {
            Type::Object { properties, .. } => {
                let resolved_type = properties[0].kind.as_property().unwrap().children[0]
                    .kind
                    .as_type()
                    .unwrap();

                assert_eq!(
                    resolved_type,
                    &Type::Identifier("Foo".to_owned(), None),
                    "The type should not be resolved because it is not exported"
                );
            }
            _ => panic!("Expected an object type"),
        }
    }

    #[test]
    fn resolves_type_aliases_in_nested_symbols_across_files_if_the_referenced_type_is_exported_later_in_the_file(
    ) {
        let foo_file = indoc! { r#"
        type Foo = string;

        export { Foo };
        "#};

        let index_file = indoc! { r#"
        import { Foo } from "./foo.ts";

        type Bar = {
            foo: Foo;
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

        match symbols[0]
            .kind
            .as_type_alias()
            .unwrap()
            .the_type()
            .kind
            .as_type()
            .unwrap()
        {
            Type::Object { properties, .. } => {
                let resolved_type = properties[0].kind.as_property().unwrap().children[0]
                    .kind
                    .as_type()
                    .unwrap();

                assert_eq!(
                    resolved_type,
                    &Type::Identifier("Foo".to_owned(), Some("foo.ts::Foo".to_owned()))
                );
            }
            _ => panic!("Expected an object type"),
        }
    }

    #[test]
    fn resolves_type_aliases_to_nearest_symbol() {
        let source = indoc! { r#"
        type Foo = string;

        function identity<Foo>(arg: Foo): Foo {
            return arg;
        }
        "#};

        let mut table = parse_file(ParserContext::new(Path::new("index.ts"), source)).unwrap();

        table.resolve_types();

        let symbols = table.all_symbols().collect::<Vec<_>>();
        assert_eq!(symbols.len(), 2);

        // Find the return type and make sure it has resolved to the FQN of the
        // type variable `Foo`, and not the symbol `Foo` that is a type alias, and
        // in a lower scope
        let return_type = symbols[1]
            .kind
            .as_function()
            .unwrap()
            .return_type()
            .unwrap()
            .kind
            .as_type()
            .unwrap();

        assert_eq!(
            return_type,
            &Type::Identifier("Foo".to_owned(), Some("index.ts::identity::Foo".to_owned()))
        );
    }
}
