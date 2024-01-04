use crate::{
    symbol::{Source, Symbol, SymbolKind},
    ParserContext,
};
use dossier_core::{
    helpers::*,
    tree_sitter::{Node, Query, QueryCursor},
    Entity, Result,
};

use indoc::indoc;
use lazy_static::lazy_static;

const QUERY_STRING: &str = indoc! {"
    (property_signature
      name: (property_identifier) @property_name
      type: (type_annotation ((_) @property_type))
    )
    "};

lazy_static! {
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Property {
    pub identifier: String,
    /// Technically will ever only have one child, the type itself, but other
    /// parts of the program will expect a slice of children so this is simpler.
    pub children: Vec<Symbol>,
}

impl Property {
    pub fn as_entity(&self, _source: &Source, _fqn: &str) -> Entity {
        unimplemented!()
    }
}

pub(crate) const NODE_KIND: &str = "property_signature";

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    assert_eq!(node.kind(), NODE_KIND);

    let mut cursor = QueryCursor::new();
    let function = cursor
        .matches(&QUERY, *node, ctx.code.as_bytes())
        .next()
        .unwrap();

    let name_node = node_for_capture("property_name", function.captures, &QUERY).unwrap();
    let type_node = node_for_capture("property_type", function.captures, &QUERY).unwrap();

    let identifier = name_node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();

    ctx.push_scope(identifier.as_str());

    let my_type = crate::types::parse(&type_node, ctx)?;

    ctx.pop_scope();

    Ok(Symbol {
        fqn: ctx.construct_fqn(&identifier),
        kind: SymbolKind::Property(Property {
            identifier,
            children: Vec::from([my_type]),
        }),
        source: Source {
            file: ctx.file.to_owned(),
            offset_start_bytes: node.start_byte(),
            offset_end_bytes: node.end_byte(),
        },
        context: ctx.symbol_context().cloned(),
    })
}
