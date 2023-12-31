use dossier_core::serde_json::json;
use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, Context, Entity, Identity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

const QUERY_STRING: &str = indoc! {"
      (property_signature 
         name: (property_identifier) @property_name
         type: (type_annotation) @property_type
      ) @property
    "};

lazy_static! {
    static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

pub(crate) fn parse_from_node(
    node: Node,
    path: &Path,
    code: &str,
    ctx: &mut Context,
) -> Result<Vec<Entity>> {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&QUERY, node, code.as_bytes());

    Ok(matches
        .into_iter()
        .map(|m| {
            let main_node = node_for_capture("property", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("property_name", m.captures, &QUERY).unwrap();
            let mut type_node = node_for_capture("property_type", m.captures, &QUERY).unwrap();
            // Go to the node that actually contains the whole type
            let mut cursor = type_node.walk();
            cursor.goto_first_child();
            cursor.goto_next_sibling();
            type_node = cursor.node();

            let interface_docs = find_docs(main_node, code).map(crate::process_comment);

            let mut members = vec![];
            let mut meta = json!({});

            members.push(parse_type(&type_node, code, path));

            if is_readonly(&main_node, code) {
                meta["readonly"] = true.into();
            }

            if is_optional(&main_node, code) {
                meta["optional"] = true.into();
            }

            let title = name_node.utf8_text(code.as_bytes()).unwrap().to_owned();
            let fqn = ctx.generate_fqn(path, [title.as_str()]);

            Entity {
                title,
                description: interface_docs.unwrap_or("".to_owned()),
                kind: "property".to_string(),
                identity: Identity::FQN(fqn),
                members,
                member_context: Some("property".to_string()),
                language: "ts".to_owned(),
                meta,
                source: Source {
                    file: path.to_owned(),
                    start_offset_bytes: main_node.start_byte(),
                    end_offset_bytes: main_node.end_byte(),
                    repository: None,
                },
            }
        })
        .collect::<Vec<_>>())
}

fn parse_type(node: &Node, code: &str, path: &Path) -> Entity {
    Entity {
        title: node.utf8_text(code.as_bytes()).unwrap().to_owned(),
        description: "".to_string(),
        kind: "type".to_string(),
        identity: Identity::FQN("TODO".to_string()),
        members: vec![],
        member_context: Some("type".to_string()),
        language: "ts".to_owned(),
        meta: json!({}),
        source: Source {
            file: path.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
            repository: None,
        },
    }
}

fn is_readonly(node: &Node, code: &str) -> bool {
    node.utf8_text(code.as_bytes())
        .unwrap()
        .starts_with("readonly")
}

// TODO: Make more robust
fn is_optional(node: &Node, code: &str) -> bool {
    node.utf8_text(code.as_bytes()).unwrap().contains("?:")
}

fn find_docs<'a>(node: Node<'a>, code: &'a str) -> Option<&'a str> {
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

#[cfg(test)]
mod test {
    /// NOTE: Properties are always inside some kind of context, so tests
    /// need to create a parent node context in order to parse them correctly.
    use super::*;
    use indoc::indoc;

    #[test]
    fn parses_properties() {
        let code = indoc! { r#"
        interface ExampleInterface {
            label: string;
            optional?: string;
            readonly age: number;
        }
        "#};

        let mut parser = dossier_core::tree_sitter::Parser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading Rust grammar");

        let tree = parser.parse(code.clone(), None).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&crate::interface::QUERY, tree.root_node(), code.as_bytes());

        let parent_captures = matches
            .into_iter()
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .captures;

        let root =
            node_for_capture("interface_body", parent_captures, &crate::interface::QUERY).unwrap();

        let properties =
            parse_from_node(root, Path::new("index.ts"), code, &mut Context::new()).unwrap();

        assert_eq!(properties.len(), 3);

        let mut property = &properties[0];
        let mut property_type = &property.members[0];

        assert_eq!(property.title, "label");
        assert_eq!(property.kind, "property");
        assert_eq!(property.member_context.as_deref(), Some("property"));
        assert_eq!(property_type.title, "string");

        property = &properties[1];
        property_type = &property.members[0];

        assert_eq!(property.title, "optional");
        assert_eq!(property.kind, "property");
        assert_eq!(property.member_context.as_deref(), Some("property"));
        assert_eq!(property.meta, json!({ "optional": true }));
        assert_eq!(property_type.title, "string");
        assert_eq!(property_type.member_context.as_deref(), Some("type"));

        property = &properties[2];
        property_type = &property.members[0];

        assert_eq!(property.title, "age");
        assert_eq!(property.kind, "property");
        assert_eq!(property.member_context.as_deref(), Some("property"));
        assert_eq!(property.meta, json!({ "readonly": true }));
        assert_eq!(property_type.title, "number");
        assert_eq!(property_type.member_context.as_deref(), Some("type"));
    }
}
