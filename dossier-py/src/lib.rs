mod class;
mod function;
mod parameter;
mod symbol;
mod types;

use dossier_core::tree_sitter::Node;
use dossier_core::Result;

use std::path::{Path, PathBuf};

use class::Class;
use function::Function;
use symbol::{ParseSymbol, Symbol, SymbolContext};

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

        let paths: Vec<PathBuf> = paths
            .into_iter()
            .map(|p| p.into().to_owned())
            .collect::<Vec<_>>();

        paths.iter().for_each(|path| {
            let code = std::fs::read_to_string(path).unwrap();
            let ctx = ParserContext::new(path, &code);

            // TODO(Nik): Handle error
            let mut results = parse_file(ctx).unwrap();

            symbols.append(&mut results);
        });

        let mut entities = vec![];
        for symbol in symbols {
            let entity = symbol.as_entity();
            entities.push(entity);
        }

        Ok(entities)
    }
}

fn init_parser() -> dossier_core::tree_sitter::Parser {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(tree_sitter_python::language())
        .expect("Error loading Python language");

    parser
}

fn parse_file(mut ctx: ParserContext) -> Result<Vec<Symbol>> {
    let mut parser = init_parser();
    let tree = parser.parse(ctx.code, None).unwrap();

    let mut cursor = tree.root_node().walk();
    assert_eq!(cursor.node().kind(), "module");
    cursor.goto_first_child();
    let mut out = vec![];

    loop {
        handle_node(cursor.node(), &mut out, &mut ctx)?;

        if !cursor.goto_next_sibling() {
            break;
        }
    }

    Ok(out)
}

fn handle_node(node: Node, out: &mut Vec<Symbol>, ctx: &mut ParserContext) -> Result<()> {
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
pub(crate) struct ParserContext<'a> {
    pub file: &'a Path,
    pub code: &'a str,
    symbol_context: Vec<SymbolContext>,
}

impl<'a> ParserContext<'a> {
    fn new(file: &'a Path, code: &'a str) -> Self {
        Self { file, code, symbol_context: vec![] }
    }

    fn file(&self) -> &Path {
        self.file.clone()
    }

    fn code(&self) -> &str {
        self.code.clone()
    }

    fn push_context(&mut self, ctx: SymbolContext) {
        self.symbol_context.push(ctx)
    }

    fn pop_context(&mut self) -> Option<SymbolContext> {
        self.symbol_context.pop()
    }

    fn symbol_context(&self) -> Option<SymbolContext> {
        self.symbol_context.last().copied()
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

        let ctx = ParserContext::new(Path::new("main.py"), source);
        let symbols = parse_file(ctx).unwrap();

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

        let ctx = ParserContext::new(Path::new("main.py"), source);
        let symbols = parse_file(ctx).unwrap();

        let function = symbols.get(0).unwrap().as_function().unwrap();
        assert_eq!(function.title, "complex");
        assert_eq!(
            function.documentation.as_deref(),
            Some("Form a complex number.")
        );
    }
}
