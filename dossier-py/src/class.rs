use dossier_core::{tree_sitter::Node, Result};

use crate::{
    function::Function,
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Class<'a> {
    pub title: &'a str,
    pub documentation: Option<String>,
    pub members: Vec<Symbol<'a>>,
}

impl<'a> Class<'a> {
    #[cfg(test)]
    fn methods(&self) -> impl Iterator<Item = &Symbol<'a>> {
        self.members.iter().filter(|s| s.as_function().is_some())
    }
}

impl<'a> ParseSymbol<'a> for Class<'a> {
    fn matches_node(node: tree_sitter::Node<'a>) -> bool {
        node.kind() == "class_definition"
    }

    fn parse_symbol(node: tree_sitter::Node<'a>, ctx: &'a ParserContext) -> Result<Symbol<'a>> {
        assert_eq!(node.kind(), "class_definition", "Expected class definition");

        let title = node
            .child_by_field_name("name")
            .expect("Expected class name")
            .utf8_text(ctx.code().as_bytes())
            .unwrap();

        let documentation = find_docs(&node, ctx);

        let mut members = vec![];

        if let Some(body) = node.child_by_field_name("body") {
            parse_methods(&body, ctx, &mut members)?;
        }

        Ok(Symbol::new(
            SymbolKind::Class(Class {
                title,
                documentation,
                members,
            }),
            Location::new(&node, ctx),
        ))
    }
}

fn parse_methods<'a>(
    node: &Node<'a>,
    ctx: &'a ParserContext,
    members: &mut Vec<Symbol<'a>>,
) -> Result<()> {
    let mut cursor = node.walk();
    cursor.goto_first_child();

    loop {
        if Function::matches_node(cursor.node()) {
            let mut method = Function::parse_symbol(cursor.node(), ctx)?;
            method.context = Some(SymbolContext::Method);
            members.push(method);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
}

fn find_docs<'a>(node: &Node<'a>, ctx: &'a ParserContext) -> Option<String> {
    if let Some(body) = node.child_by_field_name("body") {
        let mut cursor = body.walk();
        cursor.goto_first_child();

        if cursor.node().kind() == "expression_statement" {
            cursor.goto_first_child();
            if cursor.node().kind() == "string" {
                let possible_docs = cursor.node().utf8_text(ctx.code().as_bytes()).unwrap();
                crate::helpers::process_docs(possible_docs)
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;
    use std::path::PathBuf;

    #[test]
    fn parse_methods() {
        let source = indoc! {r#"
        class PyClass:
            def says(self, sound=None):
                """Prints what the animals name is and what sound it makes."""
                1 + 1
        "#};

        let ctx = ParserContext::new(PathBuf::from("test.py"), source.to_owned());

        let node = ctx.root_node();
        let mut cursor = node.walk();
        cursor.goto_first_child();

        assert!(Class::matches_node(cursor.node()));

        let symbol = Class::parse_symbol(cursor.node(), &ctx).unwrap();
        let class = symbol.as_class().unwrap();

        let method_symbol = class.methods().next().unwrap();
        let method = method_symbol.as_function().unwrap();
        assert_eq!(method.title, "says");
        assert_eq!(
            method.documentation.as_deref(),
            Some("Prints what the animals name is and what sound it makes.")
        );

        assert_eq!(method_symbol.context, Some(SymbolContext::Method));
    }
}
