use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Instant;

use dossier_core::DocsParser;

use clap::Parser;

/// Dossier: A multi-language soure code and docstring parser
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Input files to parse
    #[arg(required = true)]
    files: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse_from(wild::args());

    let start = Instant::now();

    let mut input_files = vec![];
    let mut out = vec![];

    for file in args.files {
        if file.is_dir() {
            continue;
        }

        input_files.push(file);
    }

    let typescript_files = input_files
        .iter()
        .filter(|f| f.extension() == Some(OsStr::new("ts")))
        .map(|p| p.as_path())
        .collect::<Vec<_>>();

    let parser = dossier_ts::TypeScriptParser::new();

    match parser.parse(typescript_files, &mut dossier_core::Context::new()) {
        Ok(mut entities) => {
            out.append(&mut entities);
        }
        Err(_e) => {
            eprint!("Error parsing docs");
            std::process::exit(1);
        }
    }

    let python_files = input_files
        .iter()
        .filter(|f| f.extension() == Some(OsStr::new(dossier_py::LANGUAGE)))
        .map(|p| p.as_path())
        .collect::<Vec<_>>();

    let parser = dossier_py::PythonParser::new();

    match parser.parse(python_files, &mut dossier_core::Context::new()) {
        Ok(mut entities) => {
            out.append(&mut entities);
        }
        Err(_e) => {
            eprint!("Error parsing docs");
            std::process::exit(1);
        }
    }

    let duration = start.elapsed();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    eprintln!(
        "Processed {} files in {}",
        input_files.len(),
        pretty_duration::pretty_duration(&duration, None)
    );
}
