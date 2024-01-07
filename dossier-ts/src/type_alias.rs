use crate::{
    symbol::{Source, Symbol, SymbolKind},
    types, ParserContext,
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
        &self.children[0]
    }
}

pub(crate) const NODE_KIND: &str = "type_alias_declaration";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

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

    cursor.goto_next_sibling();

    while !cursor.node().is_named() {
        cursor.goto_next_sibling();
    }

    let my_type = types::parse(&cursor.node(), ctx)?;

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::TypeAlias(TypeAlias {
            identifier,
            children: Vec::from([my_type]),
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
