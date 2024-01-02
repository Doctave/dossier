use crate::symbols::SymbolTable;
use crate::ParserContext;
use dossier_core::{tree_sitter::Node, Result};

pub(crate) const NODE_KIND: &str = "import_statement";

/// Represents an import statement.
///
/// Can be created by parsing an ES6 module import, or a CommonJS require.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Import {
    pub names: Vec<String>,
    pub source: String,
}

pub(crate) fn parse(node: &Node, table: &mut SymbolTable, ctx: &ParserContext) -> Result<()> {
    let mut cursor = node.walk();
    assert!(cursor.node().kind() == NODE_KIND);

    cursor.goto_first_child();

    // Pop the unnamed import node
    //
    // import { Foo, Bar } from 'baz';
    // ^^^^^^
    cursor.goto_next_sibling();

    // Parse the import names.
    //
    // import { Foo, Bar } from 'baz';
    //        ^^^^^^^^^^^^
    let mut import_cursor = cursor.node().walk();
    let mut names = vec![];
    // named_imports
    import_cursor.goto_first_child();
    // first import_specifier
    import_cursor.goto_first_child();

    while import_cursor.goto_next_sibling() {
        if !import_cursor.node().is_named() {
            continue;
        }
        let name = import_cursor.node().utf8_text(ctx.code.as_bytes()).unwrap();
        names.push(name.to_owned());
    }

    // Pop "from"
    cursor.goto_next_sibling();

    // Parse the source
    //
    // import { Foo, Bar } from './baz';
    cursor.goto_next_sibling();
    // Pop quote
    cursor.goto_first_child();
    cursor.goto_next_sibling();
    let source = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    table.add_import(Import { names, source });

    Ok(())
}
