use dossier_core::{Context, DocsParser, Entity, Result};

use std::path::Path;

mod class;
mod field;
mod function;
mod interface;
mod method;
mod parameter;
mod property;

pub struct Parser {}

impl DocsParser for Parser {
    fn parse(&self, path: &Path, ctx: &mut Context) -> Result<Vec<Entity>> {
        let code = std::fs::read_to_string(path).unwrap();

        let mut out = vec![];

        out.append(&mut interface::parse(&code, path, ctx)?);
        out.append(&mut class::parse(&code, path, ctx)?);
        out.append(&mut function::parse(&code, path, ctx)?);

        Ok(out)
    }
}

pub(crate) fn process_comment(comment: &str) -> String {
    let mut tmp = comment.trim().to_owned();
    tmp = tmp.trim_start_matches("/**").to_owned();
    tmp = tmp.trim_end_matches("*/").to_owned();

    tmp.lines()
        .map(|l| l.trim().trim_start_matches("* ").trim_start_matches('*'))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_owned()
}
