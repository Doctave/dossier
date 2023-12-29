use std::collections::VecDeque;
use std::path::PathBuf;

use dossier_core::DocsParser;

fn main() {
    let mut files: VecDeque<PathBuf> = std::env::args().map(PathBuf::from).collect();
    // Remove binary name
    files.pop_front();

    let mut out = vec![];

    for file in files {
        if file.is_dir() {
            continue;
        }
        match file.extension().and_then(|s| s.to_str()) {
            Some("ts") => {
                let parser = dossier_ts::Parser {};

                match parser.parse(&file, &dossier_core::Config {}) {
                    Ok(mut entities) => {
                        out.append(&mut entities);
                    }
                    Err(_e) => {
                        eprint!("Error parsing docs");
                        std::process::exit(1);
                    }
                }
            }
            Some(unknown) => {
                println!("Unsupported language `{}`", unknown);
                std::process::exit(1);
            }
            None => {
                println!("File missing extension");
                std::process::exit(1);
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}
