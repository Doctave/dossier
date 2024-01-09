mod class;
mod function;
mod symbol;

use dossier_core::tree_sitter::{Node, Parser};
use dossier_core::Result;
use tree_sitter::Tree;

use std::path::{Path, PathBuf};

use class::Class;
use function::Function;
use symbol::{ParseSymbol, Symbol};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PythonParser {}

impl PythonParser {
    pub fn new() -> Self {
        Self::default()
    }
}

pub const LANGUAGE: &str = "py";

impl dossier_core::DocsParser for PythonParser {
    fn parse<'a, P: Into<&'a Path>, T: IntoIterator<Item = P>>(
        &self,
        paths: T,
        _ctx: &mut dossier_core::Context,
    ) -> Result<Vec<dossier_core::Entity>> {
        let mut symbols = vec![];

        let ctxs = paths
            .into_iter()
            .map(|p| p.into().to_owned())
            .map(|p| (p.clone(), std::fs::read_to_string(p).unwrap()))
            .map(|(p, c)| ParserContext::new(p, c))
            .collect::<Vec<_>>();

        ctxs.iter().for_each(|ctx| {
            // TODO(Nik): Handle error
            symbols.append(&mut parse_file(ctx).unwrap());
        });

        let mut entities = vec![];
        for symbol in symbols {
            let entity = symbol.as_entity();
            entities.push(entity);
        }

        Ok(entities)
    }
}

fn parse_file(ctx: &ParserContext) -> Result<Vec<Symbol>> {
    let mut cursor = ctx.root_node().walk();
    assert_eq!(cursor.node().kind(), "module");
    cursor.goto_first_child();
    let mut out = vec![];

    loop {
        match cursor.node().kind() {
            _ => {
                handle_node(cursor.node(), &mut out, ctx)?;
            }
        }

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(out)
}

fn handle_node<'a>(
    node: Node<'a>,
    out: &mut Vec<Symbol<'a>>,
    ctx: &'a ParserContext,
) -> Result<()> {
    if Class::matches_node(node) {
        out.push(Class::parse_symbol(node, ctx).unwrap());
    } else if Function::matches_node(node) {
        out.push(Function::parse_symbol(node, ctx).unwrap());
    } else {
        println!("Unhandled node: {}", node.kind());
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) struct ParserContext {
    file: PathBuf,
    code: String,
    tree: Tree,
}

impl<'a> ParserContext {
    fn new(path: PathBuf, code: String) -> Self {
        let mut parser = Parser::new();

        parser
            .set_language(tree_sitter_python::language())
            .expect("Error loading Python grammar");

        let tree = parser.parse(&code, None).unwrap();

        Self {
            file: path,
            code,
            tree,
        }
    }

    fn code(&self) -> &str {
        &self.code
    }

    fn file(&self) -> &Path {
        &self.file
    }

    fn root_node(&'a self) -> Node<'a> {
        self.tree.root_node()
    }
}

mod helpers {
    pub(crate) fn process_docs(possible_docs: &str) -> Option<String> {
        if !possible_docs.starts_with("\"\"\"") {
            return None;
        }

        // Remove the triple quotes from the start and end of the docstring
        let trimmed_docstring = possible_docs
            .trim_start_matches("\"\"\"")
            .trim_end_matches("\"\"\"")
            .trim();

        // Split the trimmed docstring into lines
        let lines: Vec<&str> = trimmed_docstring.lines().collect();

        // Find the minimum indentation starting from the second line
        let min_indent = lines
            .iter()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .map(|line| line.len() - line.trim_start().len())
            .min()
            .unwrap_or(0);

        // Process each line, removing the minimum indentation from lines other than the first
        let parsed = lines
            .iter()
            .enumerate()
            .map(|(i, line)| {
                if i == 0 {
                    *line
                } else if line.len() > min_indent {
                    &line[min_indent..]
                } else {
                    line.trim()
                }
            })
            .collect::<Vec<&str>>()
            .join("\n");

        Some(parsed)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use indoc::indoc;

    #[test]
    fn parses_a_class() {
        let source = indoc! {r#"
        class PyClass:
            """Documentation for a class.

            More details.
                Some other stuff!
            """
        "#};

        let ctx = ParserContext::new(PathBuf::from("main.py"), source.to_owned());
        let symbols = parse_file(&ctx).unwrap();

        let class = symbols.get(0).unwrap().as_class().unwrap();
        assert_eq!(class.title, "PyClass");
        assert_eq!(
            class.documentation.as_deref(),
            Some("Documentation for a class.\n\nMore details.\n    Some other stuff!")
        );
    }

    #[test]
    fn parses_a_function() {
        let source = indoc! {r#"
        def complex(real=0.0, imag=0.0):
            """
            Form a complex number.
            """
            if imag == 0.0 and real == 0.0:
                return complex_zero
        "#};

        let ctx = ParserContext::new(PathBuf::from("main.py"), source.to_owned());
        let symbols = parse_file(&ctx).unwrap();

        let function = symbols.get(0).unwrap().as_function().unwrap();
        assert_eq!(function.title, "complex");
        assert_eq!(
            function.documentation.as_deref(),
            Some("Form a complex number.")
        );
    }
}
