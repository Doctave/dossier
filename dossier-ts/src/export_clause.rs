use crate::ParserContext;
use dossier_core::{tree_sitter::Node, Result};

pub(crate) const NODE_KIND: &str = "export_clause";

pub(crate) fn parse_exports(node: &Node, ctx: &mut ParserContext) -> Result<Vec<String>> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut out = vec![];

    let mut cursor = node.walk();
    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "export_specifier" {
            let mut specifier_cursor = cursor.node().walk();
            specifier_cursor.goto_first_child();

            while !specifier_cursor.node().is_named() {
                if !specifier_cursor.goto_next_sibling() {
                    break
                }
            }

            let identifier = specifier_cursor
                .node()
                .utf8_text(ctx.code.as_bytes())
                .unwrap();

            out.push(identifier.to_owned());
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(out)
}
