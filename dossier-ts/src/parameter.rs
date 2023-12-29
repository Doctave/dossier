use dossier_core::{serde_json::json, Config, Entity, Result, Source};
use tree_sitter::Node;

use std::path::Path;

/// Given a node describing function or method parameters, returns a list of
/// entities describing the parameters.
///
/// The `node` parameter should be the `parameters` tree-sitter node, with
/// children of `required_parameter` and `optional_parameter`.
pub(crate) fn parse_from_node(
    node: &Node,
    path: &Path,
    code: &str,
    _config: &Config,
) -> Result<Vec<Entity>> {
    let mut members = vec![];

    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let node_kind = cursor.node().kind();
            if node_kind == "required_parameter" || node_kind == "optional_parameter" {
                members.push(parse_parameter(&cursor.node(), code, path));
            }

            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    Ok(members)
}

fn parse_parameter(node: &Node, code: &str, path: &Path) -> Entity {
    let identifier_node = node.child_by_field_name("pattern").unwrap();
    let type_node = node.child_by_field_name("type").unwrap();

    let identifier_name = identifier_node.utf8_text(code.as_bytes()).unwrap();
    let mut type_name = type_node.utf8_text(code.as_bytes()).unwrap();
    let mut meta = json!({});

    if type_name.starts_with(':') {
        type_name = type_name.trim_start_matches(':').trim();
    }

    // TODO: More robust way to detect optional parameters
    if node.utf8_text(code.as_bytes()).unwrap().contains("?:") {
        meta["optional"] = true.into();
    }

    let type_entity = Entity {
        title: type_name.to_owned(),
        description: "".to_string(),
        kind: "type".to_string(),
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

    Entity {
        title: identifier_name.to_owned(),
        description: "".to_string(),
        kind: "parameter".to_string(),
        members: vec![type_entity],
        member_context: Some("parameter".to_string()),
        language: "ts".to_owned(),
        meta,
        source: Source {
            file: path.to_owned(),
            start_offset_bytes: node.start_byte(),
            end_offset_bytes: node.end_byte(),
            repository: None,
        },
    }
}
