use dossier_core::{Context, DocsParser, Entity, FileSource, FileSystem, Result};

use std::path::Path;

mod class;
mod field;
mod function;
mod interface;
mod method;
mod parameter;
mod property;

pub struct Parser<F: FileSource> {
    file_source: F,
}

impl<F: FileSource> Parser<F> {
    fn read_file(&self, path: &Path) -> std::io::Result<String> {
        self.file_source.read_file(path)
    }
}

impl Default for Parser<FileSystem> {
    fn default() -> Self {
        Self {
            file_source: FileSystem,
        }
    }
}

impl<F: FileSource> DocsParser for Parser<F> {
    fn parse<'a, P: Into<&'a Path>, T: IntoIterator<Item = P>>(
        &self,
        paths: T,
        ctx: &mut Context,
    ) -> Result<Vec<Entity>> {
        let mut out = vec![];

        for path in paths {
            let path: &Path = path.into();
            let code = self.read_file(path).unwrap();

            out.append(&mut interface::parse(&code, path, ctx)?);
            out.append(&mut class::parse(&code, path, ctx)?);
            out.append(&mut function::parse(&code, path, ctx)?);
        }

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

#[cfg(test)]
mod tests {
    use dossier_core::indexmap;
    use dossier_core::Identity;
    use dossier_core::InMemoryFileSystem;
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn resolve_type_references_in_one_file() {
        let fs = InMemoryFileSystem {
            files: indexmap::indexmap! {
                PathBuf::from("index.ts") => r#"
                    interface ExampleType {
                        foo: string
                    }

                    export function test(): ExampleType {
                        return { foo: "example" }
                    }
                "#.to_owned(),
            },
        };

        let entities = Parser { file_source: fs }
            .parse([Path::new("index.ts")], &mut Context::new())
            .unwrap();

        // The interface definition
        assert_eq!(entities[0].kind, "interface");
        assert_eq!(entities[0].title, "ExampleType");
        assert_eq!(
            entities[0].identity,
            Identity::FQN("index.ts::ExampleType".to_owned())
        );

        // The reference to that interface entity
        assert_eq!(entities[1].members[0].title, "ExampleType");
        assert_eq!(
            entities[1].members[0].member_context,
            Some("returnType".to_owned())
        );
        assert_eq!(
            entities[1].members[0].identity,
            Identity::Reference("index.js::ExampleType".to_owned())
        );
    }
}
