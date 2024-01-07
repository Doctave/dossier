use crate::{
    field,
    helpers::*,
    method,
    symbol::{Source, Symbol, SymbolKind},
    ParserContext,
};
use dossier_core::{serde_json::json, tree_sitter::Node, Entity, Identity, Result};

pub(crate) const NODE_KIND: &str = "class_declaration";

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Class {
    pub identifier: String,
    pub documentation: Option<String>,
    /// Interfaces are actually just a single object type.
    /// We forward a bunch of methods to this child object.
    pub children: Vec<Symbol>,
    pub exported: bool,
}

impl Class {
    pub(crate) fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        let mut meta = json!({});
        if self.exported {
            meta["exported"] = true.into();
        }

        Entity {
            title: self.identifier.clone(),
            description: self.documentation.as_deref().unwrap_or_default().to_owned(),
            kind: "class".to_owned(),
            identity: Identity::FQN(fqn.to_owned()),
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
    pub(crate) fn fields(&self) -> impl Iterator<Item = &Symbol> {
        self.children.iter().filter(|s| s.kind.as_field().is_some())
    }

    #[cfg(test)]
    pub(crate) fn methods(&self) -> impl Iterator<Item = &Symbol> {
        self.children
            .iter()
            .filter(|s| s.kind.as_method().is_some())
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut children = vec![];
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

    parse_class_body(&cursor.node(), ctx, &mut children)?;

    ctx.pop_fqn();
    ctx.pop_scope();

    Ok(Symbol::in_context(
        ctx,
        SymbolKind::Class(Class {
            identifier,
            documentation: find_docs(node, ctx.code).map(process_comment),
            children,
            exported: is_exported(node),
        }),
        Source::for_node(node, ctx),
    ))
}

fn parse_class_body(
    node: &Node,
    ctx: &mut ParserContext,
    children: &mut Vec<Symbol>,
) -> Result<()> {
    let mut cursor = node.walk();

    cursor.goto_first_child();

    loop {
        if cursor.node().kind() == "method_definition" {
            children.push(method::parse(&cursor.node(), ctx)?);
        }
        if cursor.node().kind() == field::NODE_KIND {
            children.push(field::parse(&cursor.node(), ctx)?);
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(())
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
