use dossier_core::tree_sitter::{Node, Query, QueryCursor};
use dossier_core::{helpers::*, serde_json::json, Context, Entity, Result, Source};
use indoc::indoc;
use lazy_static::lazy_static;

use std::path::Path;

const QUERY_STRING: &str = indoc! {"
      (public_field_definition
        (accessibility_modifier) ? @accessibility_modifier
        name: (property_identifier) @field_name
        type: (type_annotation) @field_type
        value: (_) ? @field_value
      ) @field
    "};

lazy_static! {
    pub(crate) static ref QUERY: Query =
        Query::new(tree_sitter_typescript::language_typescript(), QUERY_STRING).unwrap();
}

pub(crate) fn parse_from_node(
    node: Node,
    path: &Path,
    code: &str,
    ctx: &Context,
) -> Result<Vec<Entity>> {
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&QUERY, node, code.as_bytes());

    Ok(matches
        .into_iter()
        .map(|m| {
            let main_node = node_for_capture("field", m.captures, &QUERY).unwrap();
            let name_node = node_for_capture("field_name", m.captures, &QUERY).unwrap();
            let type_node = node_for_capture("field_type", m.captures, &QUERY).unwrap();
            let accessibility_modifier =
                node_for_capture("accessibility_modifier", m.captures, &QUERY);

            let mut type_name = type_node.utf8_text(code.as_bytes()).unwrap();

            if type_name.starts_with(':') {
                type_name = type_name.trim_start_matches(':').trim();
            }

            let mut meta = json!({});

            if let Some(accessibility_modifier) = accessibility_modifier {
                let modifier = accessibility_modifier.utf8_text(code.as_bytes()).unwrap();

                if modifier == "private" {
                    meta["private"] = true.into();
                }
            }

            if main_node
                .utf8_text(code.as_bytes())
                .unwrap()
                .trim()
                .starts_with("readonly")
            {
                meta["readonly"] = true.into();
            }

            let type_entity = Entity {
                title: type_name.to_string(),
                description: "".to_string(),
                kind: "type".to_string(),
                fqn: "TODO".to_string(),
                members: vec![],
                member_context: Some("type".to_string()),
                language: "ts".to_owned(),
                meta: json!({}),
                source: Source {
                    file: path.to_owned(),
                    start_offset_bytes: type_node.start_byte(),
                    end_offset_bytes: type_node.end_byte(),
                    repository: None,
                },
            };

            let title = name_node.utf8_text(code.as_bytes()).unwrap().to_owned();
            let fqn = ctx.generate_fqn(path, [title.as_str()]);

            Entity {
                title,
                description: "".to_owned(),
                kind: "field".to_string(),
                fqn,
                members: vec![type_entity],
                member_context: None,
                language: "ts".to_string(),
                source: Source {
                    file: path.to_owned(),
                    start_offset_bytes: main_node.start_byte(),
                    end_offset_bytes: main_node.end_byte(),
                    repository: None,
                },
                meta,
            }
        })
        .collect::<Vec<_>>())
}

#[cfg(test)]
mod test {
    use super::*;
    use dossier_core::serde_json::Value;
    use indoc::formatdoc;

    fn nodes_in_class_context(code: &str) -> Result<Vec<Entity>> {
        let context = formatdoc! {"
            class PlaceholderContext {{
                {}
            }}",
            code
        };

        let mut parser = dossier_core::tree_sitter::Parser::new();

        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading typescript grammar");

        let tree = parser.parse(context.clone(), None).unwrap();

        let mut cursor = QueryCursor::new();
        let matches = cursor.matches(&crate::class::QUERY, tree.root_node(), code.as_bytes());

        let parent_captures = matches
            .into_iter()
            .collect::<Vec<_>>()
            .first()
            .unwrap()
            .captures;

        let root = node_for_capture("class_body", parent_captures, &crate::class::QUERY).unwrap();

        parse_from_node(root, Path::new("index.ts"), &context, &Context::new())
    }

    #[test]
    fn parses_fields() {
        let fields = nodes_in_class_context(indoc! {r#"
          greeting: string;
        "#})
        .unwrap();

        assert_eq!(fields.len(), 1);

        let field = &fields[0];
        assert_eq!(field.title, "greeting");
        assert_eq!(field.kind, "field");
        assert_eq!(field.members.len(), 1);
        assert_eq!(field.members[0].title, "string");
        assert_eq!(field.members[0].kind, "type");
    }

    #[test]
    fn parses_private_fields() {
        let fields = nodes_in_class_context(indoc! {r#"
          private greeting: string;
        "#})
        .unwrap();

        assert_eq!(fields.len(), 1);

        let field = &fields[0];
        assert_eq!(field.title, "greeting");
        assert_eq!(field.kind, "field");
        assert_eq!(field.meta["private"], Value::Bool(true));
    }

    #[test]
    fn parses_readonly_fields() {
        let fields = nodes_in_class_context(indoc! {r#"
          readonly greeting: string;
        "#})
        .unwrap();

        assert_eq!(fields.len(), 1);

        let field = &fields[0];
        assert_eq!(field.title, "greeting");
        assert_eq!(field.kind, "field");
        assert_eq!(field.meta["readonly"], Value::Bool(true));
    }
}
