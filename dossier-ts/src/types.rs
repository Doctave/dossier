use std::mem;

use crate::{
    function, method, parameter,
    symbol::{Source, Symbol, SymbolContext, SymbolKind},
    type_variable, ParserContext,
};

use dossier_core::serde_json::json;
use dossier_core::{tree_sitter::Node, Entity, Identity, Result};

type ResolvedTypeFQN = String;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Type {
    Predefined(String),
    Parenthesized(Vec<Symbol>),
    Literal(String),
    /// This is the case where we have a type alias, and we need to resolve it.
    ///
    /// When the type has been resolved, the second element in the tuple will
    /// contain the FQN of the type.
    Identifier(String, Option<ResolvedTypeFQN>),
    Object {
        // TODO(Nik): What is the real identifier here?
        raw_string: String,
        properties: Vec<Symbol>,
    },
    Conditional {
        members: Vec<Symbol>,
    },
    Union {
        members: Vec<Symbol>,
    },
    Intersection {
        members: Vec<Symbol>,
    },
    GenericType {
        identifier: String,
        members: Vec<Symbol>,
    },
    Array {
        members: Vec<Symbol>,
    },
    Tuple {
        members: Vec<Symbol>,
    },
    Function {
        members: Vec<Symbol>,
    },
    Rest {
        members: Vec<Symbol>,
    },
    TypeOf(String),
    /// TODO(Nik): Parse the template literal and access its members
    /// Tree-sitter parses the literal into its parts, so we can
    /// parse the child nodes and understand their types.
    ///
    /// Problem is giving enough metadata to reconstruct the literal
    /// in e.g. a documentation setting.
    TemplateLiteral(String),
    KeyOf(Vec<Symbol>),
    ReadOnly(Vec<Symbol>),
    Lookup(Vec<Symbol>),
    Infer(Vec<Symbol>),
    This,
    Constructor {
        members: Vec<Symbol>,
    },
}

impl Type {
    /// TODO(Nik): Identifiers don't make sense in this situation?
    pub fn identifier(&self) -> &str {
        match self {
            Type::Predefined(type_name) => type_name.as_str(),
            Type::Identifier(identifier, _) => identifier.as_str(),
            Type::Object { raw_string, .. } => raw_string.as_str(),
            Type::Conditional { .. } => "conditional",
            Type::Union { .. } => "union",
            Type::Intersection { .. } => "intersection",
            Type::GenericType { identifier, .. } => identifier.as_str(),
            Type::Tuple { .. } => "tuple",
            Type::Array { .. } => "array",
            Type::Function { .. } => "function",
            Type::Rest { .. } => "rest",
            Type::Parenthesized(_) => "parenthesized",
            // TODO(Nik): Does this make sense?
            // Update: nope. It should be recursive, not a string.
            Type::TypeOf(name) => name,
            // TODO: Safely access these vecs and assume there's something there?
            Type::KeyOf(symbol) => symbol[0].identifier(),
            // TODO: Safely access these vecs and assume there's something there?
            Type::ReadOnly(symbol) => symbol[0].identifier(),
            // TODO: Safely access these vecs and assume there's something there?
            Type::Lookup(symbol) => symbol[0].identifier(),
            Type::Literal(name) => name,
            Type::Infer(_) => "infer",
            Type::This => "this",
            Type::TemplateLiteral(name) => name,
            // TODO(Nik): Give the members of the constructor type
            // explicit context so we can differentiate between the
            // left side, right side, consequence and alternative childs.
            Type::Constructor { .. } => "constructor",
        }
    }

    pub fn children(&self) -> &[Symbol] {
        match self {
            Type::Object {
                properties: fields, ..
            } => fields,
            Type::Union { members } => members,
            Type::Conditional { members } => members,
            Type::GenericType { members, .. } => members,
            Type::Array { members, .. } => members,
            Type::Tuple { members, .. } => members,
            Type::Function { members, .. } => members,
            Type::Parenthesized(nested) => nested,
            Type::KeyOf(nested) => nested,
            Type::ReadOnly(nested) => nested,
            Type::Lookup(nested) => nested,
            Type::Infer(nested) => nested,
            Type::Intersection { members } => members,
            Type::Rest { members } => members,
            Type::Constructor { members } => members,
            Type::TypeOf(_) => &[],
            Type::TemplateLiteral(_) => &[],
            Type::Predefined(_) => &[],
            Type::Identifier(_, _) => &[],
            Type::Literal(_) => &[],
            Type::This => &[],
        }
    }

    pub fn children_mut(&mut self) -> &mut [Symbol] {
        match self {
            Type::Object {
                properties: fields, ..
            } => fields,
            Type::Union { members } => members,
            Type::Conditional { members } => members,
            Type::GenericType { members, .. } => members,
            Type::Array { members, .. } => members,
            Type::Tuple { members, .. } => members,
            Type::Function { members, .. } => members,
            Type::Parenthesized(nested) => nested,
            Type::KeyOf(nested) => nested,
            Type::ReadOnly(nested) => nested,
            Type::Lookup(nested) => nested,
            Type::Infer(nested) => nested,
            Type::Intersection { members } => members,
            Type::Rest { members } => members,
            Type::Constructor { members } => members,
            Type::TypeOf(_) => &mut [],
            Type::TemplateLiteral(_) => &mut [],
            Type::Predefined(_) => &mut [],
            Type::Identifier(_, _) => &mut [],
            Type::Literal(_) => &mut [],
            Type::This => &mut [],
        }
    }

    pub fn as_entity(&self, source: &Source, fqn: &str) -> Entity {
        match &self {
            Type::This => {
                let meta = json!({});

                Entity {
                    title: String::from("this"),
                    description: String::new(),
                    kind: "this_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Rest { members } => {
                let meta = json!({});

                let title_inner = members
                    .iter()
                    .map(|s| s.identifier())
                    .collect::<Vec<_>>()
                    .join(", ");

                Entity {
                    title: format!("...{}", title_inner),
                    description: String::new(),
                    kind: "rest_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Infer(members) => {
                let meta = json!({});

                let title_inner = members
                    .iter()
                    .map(|s| s.identifier())
                    .collect::<Vec<_>>()
                    .join(", ");

                Entity {
                    title: format!("[{}]", title_inner),
                    description: String::new(),
                    kind: "infer_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Tuple { members } => {
                let meta = json!({});

                let title_inner = members
                    .iter()
                    .map(|s| s.identifier())
                    .collect::<Vec<_>>()
                    .join(", ");

                Entity {
                    title: format!("[{}]", title_inner),
                    description: String::new(),
                    kind: "tuple".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Conditional { members } => {
                let meta = json!({});

                Entity {
                    title: format!(
                        "{} extends {} ? {} : {}",
                        self.conditional_left().unwrap().identifier(),
                        self.conditional_right().unwrap().identifier(),
                        self.conditional_consequence().unwrap().identifier(),
                        self.conditional_alternative().unwrap().identifier()
                    ),
                    description: String::new(),
                    kind: "template_literal_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Lookup(members) => {
                let meta = json!({});

                Entity {
                    title: format!("{}[{}]", members[0].identifier(), members[1].identifier()),
                    description: String::new(),
                    kind: "template_literal_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::TemplateLiteral(literal) => {
                let meta = json!({});

                Entity {
                    title: literal.to_owned(),
                    description: String::new(),
                    kind: "template_literal_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::ReadOnly(nested) => {
                let mut entity = nested[0].as_entity();
                entity.meta["readonly"] = true.into();
                entity
            }
            Type::KeyOf(nested) => {
                let meta = json!({});

                Entity {
                    title: "keyof".to_owned(),
                    description: String::new(),
                    kind: "keyof".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![nested[0].as_entity()],
                }
            }
            Type::Constructor { members } => {
                let meta = json!({});

                Entity {
                    title: "constructor".to_owned(),
                    description: String::new(),
                    kind: "parenthesized_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Parenthesized(name) => {
                let meta = json!({});

                let title = if let Some(inner) = name.first() {
                    format!("({})", inner.identifier())
                } else {
                    String::from("()")
                };

                Entity {
                    title,
                    description: String::new(),
                    kind: "parenthesized_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Literal(name) => {
                let meta = json!({});

                Entity {
                    title: format!("\"{}\"", name),
                    description: String::new(),
                    kind: "literal".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::TypeOf(name) => {
                let meta = json!({});

                Entity {
                    title: format!("typeof {}", name),
                    description: String::new(),
                    kind: "typeof".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Function { members } => {
                let meta = json!({});

                Entity {
                    title: "function_type".to_owned(),
                    description: String::new(),
                    kind: "function_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Array { members } => {
                let meta = json!({});

                Entity {
                    title: "array_type".to_owned(),
                    description: String::new(),
                    kind: "array_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::GenericType {
                identifier,
                members,
            } => {
                let meta = json!({});

                Entity {
                    title: identifier.to_owned(),
                    description: String::new(),
                    kind: "generic_type".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Intersection { members } => {
                let meta = json!({});

                Entity {
                    title: "intersection".to_owned(),
                    description: String::new(),
                    kind: "intersection".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Union { members } => {
                let meta = json!({});

                Entity {
                    title: "union".to_owned(),
                    description: String::new(),
                    kind: "union".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: members.iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Object { .. } => {
                let meta = json!({});

                Entity {
                    title: "object".to_owned(),
                    description: String::new(),
                    kind: "object".to_owned(),
                    identity: Identity::FQN(fqn.to_owned()),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: self.children().iter().map(|s| s.as_entity()).collect(),
                }
            }
            Type::Predefined(type_name) => {
                let meta = json!({});

                Entity {
                    title: type_name.clone(),
                    description: String::new(),
                    kind: "predefined_type".to_owned(),
                    identity: Identity::FQN(format!("builtin::{}", type_name)),
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
            Type::Identifier(type_name, reference) => {
                let meta = json!({});

                Entity {
                    title: type_name.clone(),
                    description: String::new(),
                    kind: "predefined_type".to_owned(),
                    identity: if let Some(fqn) = reference {
                        Identity::Reference(fqn.to_owned())
                    } else {
                        Identity::FQN(fqn.to_owned())
                    },
                    member_context: None,
                    language: crate::LANGUAGE.to_owned(),
                    source: source.as_entity_source(),
                    meta,
                    members: vec![],
                }
            }
        }
    }

    #[cfg(test)]
    pub fn function_parameters(&self) -> impl Iterator<Item = &Symbol> {
        match &self {
            Type::Function { members } => {
                members.iter().filter(|s| s.kind.as_parameter().is_some())
            }
            _ => panic!("Expected a function type"),
        }
    }

    #[cfg(test)]
    pub fn function_return_type(&self) -> Option<&Symbol> {
        match &self {
            Type::Function { members } => members
                .iter()
                .find(|s| s.context == Some(crate::symbol::SymbolContext::ReturnType)),
            _ => panic!("Expected a function type"),
        }
    }

    #[cfg(test)]
    pub fn union_left(&self) -> Option<&Symbol> {
        match self {
            Type::Union { members } => members.get(0),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn union_right(&self) -> Option<&Symbol> {
        match self {
            Type::Union { members } => members.get(1),
            _ => None,
        }
    }

    pub fn conditional_left(&self) -> Option<&Symbol> {
        match self {
            Type::Conditional { members } => members.get(0),
            _ => None,
        }
    }

    pub fn conditional_right(&self) -> Option<&Symbol> {
        match self {
            Type::Conditional { members } => members.get(1),
            _ => None,
        }
    }

    pub fn conditional_consequence(&self) -> Option<&Symbol> {
        match self {
            Type::Conditional { members } => members.get(2),
            _ => None,
        }
    }

    pub fn conditional_alternative(&self) -> Option<&Symbol> {
        match self {
            Type::Conditional { members } => members.get(3),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn intersection_left(&self) -> Option<&Symbol> {
        match self {
            Type::Intersection { members } => members.get(0),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn intersection_right(&self) -> Option<&Symbol> {
        match self {
            Type::Intersection { members } => members.get(1),
            _ => None,
        }
    }

    #[cfg(test)]
    pub fn constructor_type_variables(&self) -> impl Iterator<Item = &Symbol> {
        match self {
            Type::Constructor { members } => members
                .iter()
                .filter(|s| s.kind.as_type_variable().is_some()),
            _ => panic!("Expected a constructor type"),
        }
    }

    #[cfg(test)]
    pub fn constructor_parameters(&self) -> impl Iterator<Item = &Symbol> {
        match self {
            Type::Constructor { members } => {
                members.iter().filter(|s| s.kind.as_parameter().is_some())
            }
            _ => panic!("Expected a constructor type"),
        }
    }

    pub fn resolvable_identifier(&self) -> Option<&str> {
        match self {
            Type::Identifier(identifier, _referred_fqn) => Some(identifier.as_str()),
            _ => None,
        }
    }

    pub fn resolve_type(&mut self, fqn: &str) {
        #[allow(clippy::single_match)]
        match self {
            Type::Identifier(_, referred_fqn) => {
                *referred_fqn = Some(fqn.to_owned());
            }
            _ => {}
        }
    }
}

pub(crate) fn parse(node: &Node, ctx: &mut ParserContext) -> Result<Symbol> {
    match node.kind() {
        "this_type" => Ok(Symbol::in_context(
            ctx,
            SymbolKind::Type(Type::This),
            Source::for_node(node, ctx),
        )),
        "rest_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            members.push(parse(&cursor.node(), ctx)?);

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Rest { members }),
                Source::for_node(node, ctx),
            ))
        }
        "infer_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            members.push(parse(&cursor.node(), ctx)?);

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Infer(members)),
                Source::for_node(node, ctx),
            ))
        }
        "tuple_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();

            loop {
                if cursor.node().is_named() {
                    members.push(parse(&cursor.node(), ctx)?);
                }

                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Tuple { members }),
                Source::for_node(node, ctx),
            ))
        }
        "conditional_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();

            loop {
                if !cursor.node().is_named() || cursor.node().kind() == "comment" {
                    cursor.goto_next_sibling();
                    continue;
                }

                members.push(parse(&cursor.node(), ctx)?);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Conditional { members }),
                Source::for_node(node, ctx),
            ))
        }
        "lookup_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let left = parse(&cursor.node(), ctx)?;

            cursor.goto_next_sibling();
            cursor.goto_next_sibling();

            let right = parse(&cursor.node(), ctx)?;

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Lookup(vec![left, right])),
                Source::for_node(node, ctx),
            ))
        }
        "template_literal_type" => {
            let as_string = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::TemplateLiteral(as_string)),
                Source::for_node(node, ctx),
            ))
        }
        "readonly_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            let inner = parse(&cursor.node(), ctx)?;

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::ReadOnly(vec![inner])),
                Source::for_node(node, ctx),
            ))
        }
        "index_type_query" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            let inner = parse(&cursor.node(), ctx)?;

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::KeyOf(vec![inner])),
                Source::for_node(node, ctx),
            ))
        }
        "constructor_type" => {
            let mut members = vec![];

            if let Some(params) = node.child_by_field_name("type_parameters") {
                let mut cursor = params.walk();
                cursor.goto_first_child();

                loop {
                    if cursor.node().kind() == crate::type_variable::NODE_KIND {
                        members.push(type_variable::parse(&cursor.node(), ctx)?);
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            if let Some(params) = node.child_by_field_name("parameters") {
                let mut cursor = params.walk();
                cursor.goto_first_child();

                loop {
                    if crate::parameter::NODE_KINDS.contains(&cursor.node().kind()) {
                        members.push(parameter::parse(&cursor.node(), ctx)?);
                    }

                    if !cursor.goto_next_sibling() {
                        break;
                    }
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Constructor { members }),
                Source::for_node(node, ctx),
            ))
        }
        "parenthesized_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            let inner = parse(&cursor.node(), ctx)?;

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Parenthesized(vec![inner])),
                Source::for_node(node, ctx),
            ))
        }
        "literal_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let literal = cursor
                .node()
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Literal(literal)),
                Source::for_node(node, ctx),
            ))
        }
        "type_query" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            let identifier = cursor
                .node()
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::TypeOf(identifier)),
                Source::for_node(node, ctx),
            ))
        }
        "function_type" => {
            let mut members = vec![];

            if let Some(params) = node.child_by_field_name("parameters") {
                function::parse_parameters(&params, &mut members, ctx)?;
            }
            if let Some(params) = node.child_by_field_name("return_type") {
                ctx.push_context(SymbolContext::ReturnType);
                members.push(parse(&params, ctx)?);
                ctx.pop_context()
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Function { members }),
                Source::for_node(node, ctx),
            ))
        }
        "array_type" => {
            let mut members = vec![];
            let mut cursor = node.walk();
            cursor.goto_first_child();

            members.push(parse(&cursor.node(), ctx)?);

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Array { members }),
                Source::for_node(node, ctx),
            ))
        }
        "generic_type" => {
            let identifier = node
                .child_by_field_name("name")
                .unwrap()
                .utf8_text(ctx.code.as_bytes())
                .unwrap()
                .to_owned();

            let mut members = vec![];
            for arg in node.children_by_field_name("type_arguments", &mut node.walk()) {
                let mut cursor = arg.walk();
                cursor.goto_first_child();
                cursor.goto_next_sibling();

                ctx.push_fqn(&identifier);

                members.push(parse(&cursor.node(), ctx)?);

                ctx.pop_fqn();
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::GenericType {
                    identifier,
                    members,
                }),
                Source::for_node(node, ctx),
            ))
        }
        "predefined_type" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Predefined(type_name)),
                Source::for_node(node, ctx),
            ))
        }
        "type_identifier" => {
            let type_name = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Identifier(type_name, None)),
                Source::for_node(node, ctx),
            ))
        }
        "object_type" => {
            let type_as_string = node.utf8_text(ctx.code.as_bytes()).unwrap().to_owned();
            let mut properties = vec![];

            ctx.push_context(SymbolContext::Property);

            let mut cursor = node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();

            loop {
                if cursor.node().kind() == crate::property::NODE_KIND {
                    let symbol = crate::property::parse(&cursor.node(), ctx)?;
                    properties.push(symbol);
                }
                if cursor.node().kind() == method::NODE_KIND {
                    let symbol = method::parse(&cursor.node(), ctx)?;
                    properties.push(symbol);
                }
                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            ctx.pop_scope();

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Object {
                    raw_string: type_as_string,
                    properties,
                }),
                Source::for_node(node, ctx),
            ))
        }
        "union_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let mut members = vec![];

            loop {
                if cursor.node().kind() == "|" || cursor.node().kind() == "comment" {
                    cursor.goto_next_sibling();
                    continue;
                }
                members.push(parse(&cursor.node(), ctx)?);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Union { members }),
                Source::for_node(node, ctx),
            ))
        }
        "intersection_type" => {
            let mut cursor = node.walk();
            cursor.goto_first_child();

            let mut members = vec![];

            loop {
                if cursor.node().kind() == "&" || cursor.node().kind() == "comment" {
                    cursor.goto_next_sibling();
                    continue;
                }
                members.push(parse(&cursor.node(), ctx)?);

                if !cursor.goto_next_sibling() {
                    break;
                }
            }

            Ok(Symbol::in_context(
                ctx,
                SymbolKind::Type(Type::Intersection { members }),
                Source::for_node(node, ctx),
            ))
        }
        _ => panic!(
            "Unhandled type kind: {} | {} | {} | file:{} | pos:{}",
            node.kind(),
            node.utf8_text(ctx.code.as_bytes()).unwrap(),
            node.to_sexp(),
            ctx.file.display(),
            node.start_position()
        ),
    }
}

#[cfg(test)]
mod test {
    /// NOTE ABOUT THESE TESTS
    ///
    /// You'll notice that these tests define examples code snippets that
    /// contain a type alias (e.g. `type Foo = string;`). This is because
    /// a type definition is not valid on its own, and the parser will fail.
    /// We need the type to be in some kind of context.
    ///
    /// So each test will setup their own context, move the cursor to the
    /// point where the actual type starts, and then parse only the type.
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

    fn walk_tree_to_type(cursor: &mut TreeCursor) {
        cursor.goto_first_child();
        cursor.goto_first_child();
        cursor.goto_next_sibling();
        cursor.goto_next_sibling();
        cursor.goto_next_sibling();
    }

    #[test]
    fn parses_predefined_type() {
        let code = indoc! {r#"
            type Foo = string;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Predefined(type_name) => {
                assert_eq!(type_name, "string");
            }
            _ => panic!("Expected a predefined type"),
        }
    }

    #[test]
    fn parses_type_identifiers() {
        let code = indoc! {r#"
            type Foo = Bar;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Identifier(type_name, referred_fqn) => {
                assert_eq!(type_name, "Bar");
                assert_eq!(
                    referred_fqn, &None,
                    "The type should not be resolved at this point"
                );
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_object_types() {
        let code = indoc! {r#"
            type Player = {
                name: string;
                age?: number;
            }
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Object { properties, .. } => {
                assert_eq!(properties.len(), 2);

                assert_eq!(properties[0].kind.as_property().unwrap().identifier, "name");
                assert!(!properties[0].kind.as_property().unwrap().optional);
                assert_eq!(
                    properties[0].kind.as_property().unwrap().children[0]
                        .kind
                        .as_type()
                        .unwrap(),
                    &Type::Predefined("string".to_string())
                );
                assert_eq!(properties[1].kind.as_property().unwrap().identifier, "age");
                assert!(properties[1].kind.as_property().unwrap().optional);
                assert_eq!(
                    properties[1].kind.as_property().unwrap().children[0]
                        .kind
                        .as_type()
                        .unwrap(),
                    &Type::Predefined("number".to_string())
                );
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_union_type() {
        let code = indoc! {r#"
            type Foo = string | number;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Union { .. } => {
                let left = type_def.union_left().unwrap().kind.as_type().unwrap();
                assert_eq!(left, &Type::Predefined("string".to_string()));

                let right = type_def.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(right, &Type::Predefined("number".to_string()));
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_nested_union_type() {
        let code = indoc! {r#"
            type Foo = string | number | boolean;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Union { .. } => {
                let left = type_def.union_left().unwrap().kind.as_type().unwrap();
                let left_left = left.union_left().unwrap().kind.as_type().unwrap();
                assert_eq!(left_left, &Type::Predefined("string".to_string()));

                let left_right = left.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(left_right, &Type::Predefined("number".to_string()));

                let right = type_def.union_right().unwrap().kind.as_type().unwrap();
                assert_eq!(right, &Type::Predefined("boolean".to_string()));
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_intersection_type() {
        let code = indoc! {r#"
            type Foo = Bar & Baz;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        match type_def {
            Type::Intersection { .. } => {
                let left = type_def
                    .intersection_left()
                    .unwrap()
                    .kind
                    .as_type()
                    .unwrap();
                assert_eq!(left, &Type::Identifier("Bar".to_string(), None));

                let right = type_def
                    .intersection_right()
                    .unwrap()
                    .kind
                    .as_type()
                    .unwrap();
                assert_eq!(right, &Type::Identifier("Baz".to_string(), None));
            }
            _ => panic!("Expected a type identifier"),
        }
    }

    #[test]
    fn parses_generic_type() {
        let code = indoc! {r#"
            type Foo = Promise<Example>;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert_eq!(type_def.identifier(), "Promise");
        assert_eq!(type_def.children().len(), 1);
        assert!(matches!(type_def, Type::GenericType { .. }));

        let arg = type_def.children()[0].kind.as_type().unwrap();
        assert_eq!(arg, &Type::Identifier("Example".to_owned(), None));
    }

    #[test]
    fn parses_array_type() {
        let code = indoc! {r#"
            type Foo = string[];
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert!(matches!(type_def, Type::Array { .. }));

        assert_eq!(type_def.children().len(), 1);

        let arg = type_def.children()[0].kind.as_type().unwrap();
        assert_eq!(arg, &Type::Predefined("string".to_owned()));
    }

    #[test]
    fn parses_function_type() {
        let code = indoc! {r#"
            type Foo = (a: string) => void;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert!(matches!(type_def, Type::Function { .. }));

        assert_eq!(type_def.children().len(), 2);

        let param = type_def.function_parameters().next().unwrap();
        assert_eq!(param.kind.as_parameter().unwrap().identifier, "a");

        let return_type = type_def.function_return_type().unwrap();
        assert_eq!(
            return_type.kind.as_type().unwrap(),
            &Type::Predefined("void".to_owned())
        );
    }

    #[test]
    fn parses_typeof() {
        let code = indoc! {r#"
            type Request = typeof TediousRequest
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert_eq!(type_def, &Type::TypeOf("TediousRequest".to_owned()));
    }

    #[test]
    fn parses_literal_type() {
        let code = indoc! {r#"
            type Request = "FOO";
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert_eq!(type_def, &Type::Literal("\"FOO\"".to_owned()));
    }

    #[test]
    fn parses_parenthesized_type() {
        let code = indoc! {r#"
            type Foo = (string);
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert!(matches!(type_def, &Type::Parenthesized(_)));

        let child = type_def.children()[0].kind.as_type().unwrap();
        assert!(matches!(child, &Type::Predefined(_)));
    }

    #[test]
    fn parses_constructor_type() {
        let code = indoc! {r#"
        type PostgresCursorConstructor = new <T>(
          sql: string,
          parameters: unknown[]
        ) => PostgresCursor<T>
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert!(matches!(type_def, &Type::Constructor { .. }));

        let type_variable = type_def.constructor_type_variables().next().unwrap();
        assert_eq!(
            type_variable.kind.as_type_variable().unwrap().identifier,
            "T"
        );

        let parameters = type_def.constructor_parameters().collect::<Vec<_>>();
        assert_eq!(parameters.len(), 2);
    }

    #[test]
    fn parses_readonly_type() {
        let code = indoc! {r#"
            type Example = readonly string;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let the_type = symbol.kind.as_type().unwrap();
        assert!(matches!(the_type, Type::ReadOnly(_)));

        let inner = the_type.children()[0].kind.as_type().unwrap();

        assert!(matches!(inner, Type::Predefined(_)));
        assert_eq!(inner.identifier(), "string");
    }

    #[test]
    fn parses_template_literal_type() {
        let code = indoc! {r#"
            type Example = `varchar(${number})`;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let the_type = symbol.kind.as_type().unwrap();
        assert!(matches!(the_type, Type::TemplateLiteral(_)));

        assert_eq!(the_type.identifier(), "`varchar(${number})`");
    }

    #[test]
    fn parses_lookup_type() {
        let code = indoc! {r#"
            type Example = Foo["example"];
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let the_type = symbol.kind.as_type().unwrap();
        assert!(matches!(the_type, Type::Lookup { .. }));

        let base = the_type.children()[0].kind.as_type().unwrap();
        assert_eq!(base.identifier(), "Foo");
        assert!(
            matches!(base, Type::Identifier(_, None)),
            "Expected an unresolved identifier, got {:?}",
            base
        );

        let key = the_type.children()[1].kind.as_type().unwrap();
        assert_eq!(key.identifier(), "\"example\"");
        assert!(
            matches!(key, Type::Literal(_)),
            "Expected a literal, got {:?}",
            base
        );
    }

    #[test]
    fn parses_infer_type() {
        let code = indoc! {r#"
            type Example = infer A;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let the_type = symbol.kind.as_type().unwrap();
        assert!(matches!(the_type, Type::Infer { .. }));
        assert_eq!(the_type.identifier(), "infer");

        assert_eq!(the_type.children().len(), 1);
        let child = the_type.children()[0].kind.as_type().unwrap();
        assert!(matches!(child, Type::Identifier(_, None)));
        assert_eq!(child.identifier(), "A");
    }

    #[test]
    fn parses_tuple_type() {
        let code = indoc! {r#"
            type Example = [string, number, boolean];
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let the_type = symbol.kind.as_type().unwrap();
        assert!(matches!(the_type, Type::Tuple { .. }));

        assert_eq!(the_type.children().len(), 3);

        let child = the_type.children()[0].kind.as_type().unwrap();
        assert!(matches!(child, Type::Predefined(_)));
        assert_eq!(child.identifier(), "string");

        let child = the_type.children()[1].kind.as_type().unwrap();
        assert!(matches!(child, Type::Predefined(_)));
        assert_eq!(child.identifier(), "number");

        let child = the_type.children()[2].kind.as_type().unwrap();
        assert!(matches!(child, Type::Predefined(_)));
        assert_eq!(child.identifier(), "boolean");
    }

    #[test]
    fn parses_rest_type() {
        let code = indoc! {r#"
            type Foo = [...string];
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let tuple = symbol.kind.as_type().unwrap();
        assert!(matches!(tuple, Type::Tuple { .. }));

        let the_type = tuple.children()[0].kind.as_type().unwrap();
        assert!(matches!(the_type, Type::Rest { .. }));

        let child = the_type.children()[0].kind.as_type().unwrap();
        assert!(matches!(child, Type::Predefined(_)));
        assert_eq!(child.identifier(), "string");
    }

    #[test]
    fn parses_this_type() {
        let code = indoc! {r#"
            type Foo = this;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let this = symbol.kind.as_type().unwrap();
        assert!(matches!(this, Type::This));
    }

    #[test]
    fn parses_conditional_type() {
        let code = indoc! {r#"
            type Example = Dog extends Animal ? number : string;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let conditional = symbol.kind.as_type().unwrap();
        assert!(matches!(conditional, Type::Conditional { .. }));

        assert_eq!(conditional.children().len(), 4);

        let left = conditional
            .conditional_left()
            .unwrap()
            .kind
            .as_type()
            .unwrap();
        assert_eq!(left.identifier(), "Dog");
        assert!(matches!(left, Type::Identifier(_, None)));

        let right = conditional
            .conditional_right()
            .unwrap()
            .kind
            .as_type()
            .unwrap();
        assert_eq!(right.identifier(), "Animal");
        assert!(matches!(right, Type::Identifier(_, None)));

        let right = conditional
            .conditional_consequence()
            .unwrap()
            .kind
            .as_type()
            .unwrap();
        assert_eq!(right.identifier(), "number");
        assert!(matches!(right, Type::Predefined(_)));

        let right = conditional
            .conditional_alternative()
            .unwrap()
            .kind
            .as_type()
            .unwrap();
        assert_eq!(right.identifier(), "string");
        assert!(matches!(right, Type::Predefined(_)));
    }

    #[test]
    fn bug_parses_conditional_type_with_comments() {
        let code = indoc! {r#"
            type Example = Dog extends Animal ?
              number
              // Some comment
              : string;
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let conditional = symbol.kind.as_type().unwrap();
        assert!(matches!(conditional, Type::Conditional { .. }));

        assert_eq!(conditional.children().len(), 4);
    }

    #[test]
    fn bug_unbalanced_union() {
        let code = indoc! {r#"
            type Example = | number
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse
        let symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();

        let type_def = symbol.kind.as_type().unwrap();

        assert!(matches!(type_def, Type::Union { .. }));

        assert_eq!(type_def.children().len(), 1);
    }

    #[test]
    fn bug_comment_between_union() {
        let code = indoc! {r#"
            type Example =
            | number
            // This is a comment
            | string
        #"#};

        // Setup
        let tree = init_parser().parse(code, None).unwrap();
        let mut cursor = tree.root_node().walk();
        walk_tree_to_type(&mut cursor);

        // Parse successfully
        let _symbol = parse(
            &cursor.node(),
            &mut ParserContext::new(Path::new("index.ts"), code),
        )
        .unwrap();
    }
}
