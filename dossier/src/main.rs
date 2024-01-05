use std::collections::VecDeque;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::time::Instant;

use dossier_core::DocsParser;

use glob::glob;

fn main() {
    let start = Instant::now();
    let mut files: VecDeque<String> = std::env::args().map(String::from).collect();
    // Remove binary name
    files.pop_front();

    let mut input_files = vec![];
    let mut out = vec![];

    for file in files {
        for entry in glob(&file).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let file = PathBuf::from(&path);
                    if file.is_dir() {
                        continue;
                    }

                    input_files.push(file);
                }
                Err(e) => println!("{:?}", e),
            }
        }
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

    let duration = start.elapsed();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    eprintln!(
        "Processed {} files in {}",
        input_files.len(),
        pretty_duration::pretty_duration(&duration, None)
    );
}
