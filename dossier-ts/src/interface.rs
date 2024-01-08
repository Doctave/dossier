use crate::{
    helpers::*,
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    type_variable, types, ParserContext,
};
use dossier_core::{serde_json::json, tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "interface_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Interface {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Interfaces are actually just a single object type.
    /// We forward a bunch of methods to this child object.
    pub children: Vec<Symbol>,
    pub exported: bool,
}

impl Interface {
    pub fn as_entity(&self, source: &Source, fqn: Option<&str>) -> Entity {
        let mut meta = json!({});
        if self.exported {
            meta["exported"] = true.into();
        }

        Entity {
            title: Some(self.identifier.clone()),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "interface".to_owned(),
            identity: Identity::FQN(fqn.expect("Interface without FQN").to_owned()),
            member_context: None,
            language: "ts".to_owned(),
            source: source.as_entity_source(),
            meta,
            members: self
                .children
                .iter()
                .map(|s| s.as_entity())
                .collect::<Vec<_>>(),
        }
    }

    #[cfg(test)]
    pub fn type_variables(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_type_variable().is_some())
    }

    #[cfg(test)]
    /// Not actually the properties of the interface, but the properties of the
    /// object type that the interface is forwarding to.
    pub fn properties(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .find(|s| s.kind.as_type().is_some())
            .unwrap()
            .children()
            .iter()
            .filter(|s| s.kind.as_property().is_some())
    }

    #[cfg(test)]
    /// Not actually the properties of the interface, but the properties of the
    /// object type that the interface is forwarding to.
    pub fn methods(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .find(|s| s.kind.as_type().is_some())
            .unwrap()
            .children()
            .iter()
            .filter(|s| s.kind.as_method().is_some())
    }

    #[cfg(test)]
    pub fn extends(&self) -> Option<&Symbol> {
        self.children
            .iter()
            .find(|s| s.context == Some(SymbolContext::Extends))
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut children = vec![];
    let mut has_generics = false;
    let mut cursor = node.walk();

    cursor.goto_first_child();
    cursor.goto_next_sibling();

    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    ctx.push_scope();
    ctx.push_fqn(&identifier);

    cursor.goto_next_sibling();

    if cursor.node().kind() == "type_parameters" {
        parse_type_parameters(&cursor.node(), &mut children, ctx);
        ctx.push_scope();
        has_generics = true;
    }

    cursor.goto_next_sibling();

    if cursor.node().kind() == "extends_type_clause" {
        let mut tmp = cursor.node().walk();
        tmp.goto_first_child();
        tmp.goto_next_sibling();
        ctx.push_context(SymbolContext::Extends);
        let extends = types::parse(&tmp.node(), ctx)?;
        ctx.pop_context();
        children.push(extends);

        cursor.goto_next_sibling();
    }

    debug_assert_eq!(cursor.node().kind(), "object_type");

    children.push(types::parse(&cursor.node(), ctx)?);

    ctx.pop_fqn();
    ctx.pop_scope();
    if has_generics {
        ctx.pop_scope();
    }

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Interface(Interface {
            identifier,
            documentation: find_docs(node, ctx.code).map(process_comment),
            children,
            exported: is_exported(node),
        }),
        Source::for_node(node, ctx),
    ))
}

fn find_docs<'a>(node: &Node<'a>, code: &'a str) -> Option<&'a str> {
    let parent = node.parent().unwrap();

    if parent.kind() == "export_statement" {
        if let Some(maybe_comment) = parent.prev_sibling() {
            if maybe_comment.kind() == "comment" {
                return Some(maybe_comment.utf8_text(code.as_bytes()).unwrap());
            }
        }
    } else if let Some(maybe_comment) = node.prev_sibling() {
        if maybe_comment.kind() == "comment" {
            return Some(maybe_comment.utf8_text(code.as_bytes()).unwrap());
        }
    }

    None
}

fn is_exported(node: &Node) -> bool {
    if let Some(parent) = node.parent() {
        if parent.kind() == "export_statement" {
            return true;
        }
    }
    false
}

fn parse_type_parameters(
    type_parameters: &Node,
    children: &mut Vec<Symbol>,
    ctx: &mut ParserContext,
) {
    assert_eq!(type_parameters.kind(), "type_parameters");

    let mut cursor = type_parameters.walk();
    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "type_parameter" {
            let type_variable = type_variable::parse(&cursor.node(), ctx).unwrap();
            children.push(type_variable);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::method;
    use crate::types::Type;
    use dossier_core::tree_sitter::Parser;
    use dossier_core::tree_sitter::TreeCursor;
    use indoc::indoc;
    use std::path::Path;

    fn init_parser() -> Parser {
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        parser
    }

    fn walk_tree_to_interface(cursor: &mut TreeCursor) {
        assert_eq!(cursor.node().kind(), "program");
        cursor.goto_first_child();
        loop {
            if cursor.node().kind() == "interface_declaration" {
                break;
            }
            if cursor.node().kind() == "export_statement" {
                cursor.goto_first_child();
                cursor.goto_next_sibling();
                break;
            }

            if !cursor.goto_next_sibling() {
                panic!("Could not find interface_declaration node");
            }
        }
    }

    #[test]
    fn documentation() {
        let code = indoc! {r#"
        /**
         * This is a test interface.
         */
        interface Test {
            test: string;
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert_eq!(
            symbol.kind.as_interface().unwrap().documentation,
            Some("This is a test interface.".to_owned())
        );
    }

    #[test]
    fn exported() {
        let code = indoc! {r#"
        /**
         * This is a test interface.
         */
        export interface Test {
            test: string;
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        assert!(
            symbol.kind.as_interface().unwrap().exported,
            "Should be exported"
        );
    }

    #[test]
    fn generics() {
        let code = indoc! {r#"
        interface KeyValue<K, V extends string> {
          key: K,
          value: V
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let interface = symbol.kind.as_interface().unwrap();
        let generics = interface.type_variables().collect::<Vec<_>>();
        assert_eq!(generics.len(), 2);

        assert!(symbol.scope_id < generics[0].scope_id);
        let type_var = generics[0].kind.as_type_variable().unwrap();
        assert_eq!(type_var.identifier, "K");
        assert_eq!(type_var.constraints().count(), 0);

        let type_var = generics[1].kind.as_type_variable().unwrap();
        assert_eq!(type_var.identifier, "V");
        let constraint = type_var.constraints().next().unwrap();
        assert_eq!(
            constraint
                .kind
                .as_type_constraint()
                .unwrap()
                .the_type()
                .kind
                .as_type()
                .unwrap(),
            &Type::Predefined("string".to_owned())
        );

        let property = interface.properties().collect::<Vec<_>>()[0];
        assert!(generics[0].scope_id < property.scope_id);
    }

    #[test]
    fn method_signatures() {
        let code = indoc! {r#"
        interface Test {
            toOperationNode(): AliasNode
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();
        let interface = symbol.kind.as_interface().unwrap();

        assert_eq!(interface.methods().count(), 1);
        let method = interface.methods().next().unwrap();

        assert_eq!(
            method.kind.as_method().unwrap().identifier,
            method::Identifier::Name("toOperationNode".to_string())
        );

        let return_type = method.kind.as_method().unwrap().return_type().unwrap();

        assert_eq!(
            return_type.kind.as_type().unwrap(),
            &Type::Identifier("AliasNode".to_owned(), None)
        );
    }

    #[test]
    fn extends_syntax() {
        let code = indoc! {r#"
        export interface Expression<T> extends OperationNodeSource {
        }
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_interface(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let interface = symbol.kind.as_interface().unwrap();

        let extends = interface.extends().unwrap().kind.as_type().unwrap();
        assert_eq!(
            extends,
            &Type::Identifier("OperationNodeSource".to_owned(), None)
        );
    }
}
