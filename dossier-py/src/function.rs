use dossier_core::{serde_json::json, tree_sitter::Node, Context, Entity, Result};

use crate::{
    parameter::{self, Parameter},
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    ParserContext,
};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Function {
    pub title: String,
    pub documentation: Option<String>,
    pub members: Vec<Symbol>,
}

impl Function {
    pub fn as_entity(&self, loc: &Location, context: Option<&SymbolContext>) -> Entity {
        Entity {
            title: Some(self.title.to_owned()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "function".to_owned(),
            identity: dossier_core::Identity::FQN("TODO".to_owned()),
            members: vec![],
            member_context: context.map(|_| "method".to_owned()),
            language: crate::LANGUAGE.to_owned(),
            source: loc.as_source(),
            meta: json!({}),
        }
    }

    #[cfg(test)]
    fn parameters(&self) -> impl Iterator<Item = &Symbol> {
        self.members.iter().filter(|s| s.as_parameter().is_some())
    }
}

impl ParseSymbol for Function {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "function_definition"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &ParserContext) -> Result<Symbol> {
        assert_eq!(
            node.kind(),
            "function_definition",
            "Expected function definition"
        );

        let mut members = vec![];

        let title = node
            .child_by_field_name("name")
            .expect("Expected class name")
            .utf8_text(ctx.code().as_bytes())
            .unwrap()
            .to_owned();

        if let Some(parameters_node) = node.child_by_field_name("parameters") {
            parse_parameters(&parameters_node, &mut members, ctx)?;
        }

        let documentation = find_docs(&node, ctx);

        Ok(Symbol::new(
            SymbolKind::Function(Function {
                title,
                documentation,
                members,
            }),
            Location::new(&node, ctx),
        ))
    }
}

fn parse_parameters(node: &Node, out: &mut Vec<Symbol>, ctx: &ParserContext) -> Result<()> {
    let mut cursor = node.walk();
    cursor.goto_first_child();

    loop {
        if Parameter::matches_node(cursor.node()) {
            let symbol = Parameter::parse_symbol(cursor.node(), ctx)?;
            out.push(symbol);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
}

fn find_docs(node: &Node, ctx: &ParserContext) -> Option<String> {
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
    use std::path::Path;

    fn init_parser() -> tree_sitter::Parser {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(tree_sitter_python::language())
            .expect("Error loading Python language");

        parser
    }

    #[test]
    fn parse_function_params() {
        let source = indoc! {r#"
            def foo(bar, baz: int) -> bool:
                pass
        "#};

        let ctx = ParserContext::new(Path::new("test.py"), source);
        let tree = crate::init_parser().parse(source, None).unwrap();
        let mut cursor = tree.root_node().walk();
        cursor.goto_first_child();

        let symbol = Function::parse_symbol(cursor.node(), &ctx).unwrap();

        let function = symbol.as_function().unwrap();
        assert_eq!(function.title, "foo");

        let params = function.parameters().collect::<Vec<_>>();
        assert_eq!(params.len(), 2);
    }
}