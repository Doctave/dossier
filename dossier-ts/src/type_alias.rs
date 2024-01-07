use crate::{
    symbol::{Source, Symbol, SymbolKind},
    type_variable, types, ParserContext,
};
use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Result};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TypeAlias {
    pub identifier: String,
    /// Technically will ever only have one child, the type itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
    pub exported: bool,
}

impl TypeAlias {
    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        let mut meta = json!({});
        if self.exported {
            meta["exported"] = true.into();
        }

        Entity {
            title: self.identifier.clone(),
            description: String::new(),
            kind: "type_alias".to_owned(),
            identity: dossier_core::Identity::FQN(fqn.to_owned()),
            members: vec![],
            member_context: None,
            language: crate::LANGUAGE.to_owned(),
            source: dossier_core::Source {
                file: source.file.to_owned(),
                start_offset_bytes: source.start_offset_bytes,
                end_offset_bytes: source.end_offset_bytes,
                repository: None,
            },
            meta: json!({}),
        }
    }

    #[cfg(test)]
    pub fn the_type(&self) -> &Symbol {
        self.children.iter().find(|s| s.kind.as_type().is_some()).unwrap()
    }

    #[cfg(test)]
    pub fn type_variables(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_type_variable().is_some())
    }
}

pub(crate) const NODE_KIND: &str = "type_alias_declaration";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut children = vec![];
    let mut cursor = node.walk();
    cursor.goto_first_child();

    while !cursor.node().is_named() {
        cursor.goto_next_sibling();
    }

    let identifier = cursor
        .node()
        .utf8_text(ctx.code.as_bytes())
        .unwrap()
        .to_owned();

    if let Some(value) = node.child_by_field_name("value") {
        children.push(types::parse(&value, ctx)?);
    }

    if let Some(params) = node.child_by_field_name("type_parameters") {
        let mut cursor = params.walk();
        cursor.goto_first_child();

        loop {
            if cursor.node().kind() == crate::type_variable::NODE_KIND {
                children.push(type_variable::parse(&cursor.node(), ctx)?);
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::TypeAlias(TypeAlias {
            identifier,
            children,
            exported: is_exported(node),
        }),
        Source {
            file: ctx.file.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
        },
    ))
}

fn is_exported(node: &Node) -> bool {
    node.parent()
        .map(|p| p.kind() == "export_statement")
        .unwrap_or(false)
}

#[cfg(test)]
mod test {
    use super::*;
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

    fn walk_tree_to_alias(cursor: &mut TreeCursor) {
        assert_eq!(cursor.node().kind(), "program");
        cursor.goto_first_child();
        loop {
            if cursor.node().kind() == "type_alias_declaration" {
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
        type Example<T> = T;
        "#};

        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_alias(&mut cursor);

        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_variables = symbol
            .kind
            .as_type_alias()
            .unwrap()
            .type_variables()
            .collect::<Vec<_>>();
        assert_eq!(type_variables.len(), 1);

        let var = type_variables[0];
        assert_eq!(var.kind.as_type_variable().unwrap().identifier, "T");
    }
}
