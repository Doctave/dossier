use dossier_core::{serde_json::json, tree_sitter::Node, Entity, Result};

use crate::{
    parameter::Parameter,
    symbol::{Location, ParseSymbol, Symbol, SymbolContext, SymbolKind},
    types::Type,
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

    #[cfg(test)]
    fn return_type(&self) -> Option<&Symbol> {
        self.members.iter().find(|s| s.as_type().is_some())
    }
}

impl ParseSymbol for Function {
    fn matches_node(node: tree_sitter::Node) -> bool {
        node.kind() == "function_definition"
    }

    fn parse_symbol(node: tree_sitter::Node, ctx: &mut ParserContext) -> Result<Symbol> {
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
            ctx.push_context(SymbolContext::Parameter);
            ctx.push_fqn(&title);
            parse_parameters(&parameters_node, &mut members, ctx)?;
            ctx.pop_fqn();
            ctx.pop_context();
        }

        if let Some(return_type_node) = node.child_by_field_name("return_type") {
            ctx.push_context(SymbolContext::ReturnType);
            if Type::matches_node(return_type_node) {
                let symbol = Type::parse_symbol(return_type_node, ctx)?;
                members.push(symbol);
            }
            ctx.pop_context();
        }

        let documentation = find_docs(&node, ctx);

        Ok(Symbol::in_context(
            ctx,
            SymbolKind::Function(Function {
                title,
                documentation,
                members,
            }),
            Location::new(&node, ctx),
        ))
    }
}

fn parse_parameters(node: &Node, out: &mut Vec<Symbol>, ctx: &mut ParserContext) -> Result<()> {
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
    use crate::types::Type;

    use super::*;
    use crate::symbol::SymbolContext;
    use indoc::indoc;
    use std::path::Path;

    #[test]
    fn parse_function_params() {
        let source = indoc! {r#"
            def foo(bar, baz: int) -> bool:
                pass
        "#};

        let mut ctx = ParserContext::new(Path::new("test.py"), source);
        let tree = crate::init_parser().parse(source, None).unwrap();
        let mut cursor = tree.root_node().walk();
        cursor.goto_first_child();

        let symbol = Function::parse_symbol(cursor.node(), &mut ctx).unwrap();
        assert_eq!(symbol.fqn.as_deref(), Some("test.py::foo"));

        let function = symbol.as_function().unwrap();
        assert_eq!(function.title, "foo");

        let params = function.parameters().collect::<Vec<_>>();
        assert_eq!(params.len(), 2);

        let param = params[0].as_parameter().unwrap();
        assert_eq!(params[0].context, Some(SymbolContext::Parameter));
        assert_eq!(params[0].fqn.as_deref(), Some("test.py::foo::bar"));
        assert_eq!(param.title, "bar");
        assert_eq!(param.the_type(), None);

        let param = params[1].as_parameter().unwrap();
        assert_eq!(params[1].context, Some(SymbolContext::Parameter));
        assert_eq!(params[1].fqn.as_deref(), Some("test.py::foo::baz"));
        assert_eq!(param.title, "baz");
        let the_type = param.the_type();
        assert_eq!(
            the_type.unwrap().as_type().unwrap(),
            &Type::BuiltIn("int".to_owned())
        );

        let return_type = function.return_type().unwrap();
        assert_eq!(return_type.context, Some(SymbolContext::ReturnType));
        assert_eq!(
            return_type.as_type().unwrap(),
            &Type::BuiltIn("bool".to_owned())
        );
    }
}
