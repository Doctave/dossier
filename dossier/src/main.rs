use std::collections::VecDeque;
use std::path::PathBuf;

use dossier_core::DocsParser;

fn main() {
    let mut files: VecDeque<PathBuf> = std::env::args().map(PathBuf::from).collect();
    // Remove binary name
    files.pop_front();

    for file in files {
        match file.extension().and_then(|s| s.to_str()) {
            Some("ts") => {
                let parser = dossier_ts::Parser {};

                match parser.parse(&file, &dossier_core::Config {}) {
                    Ok(entities) => {
                        println!("{}", serde_json::to_string_pretty(&entities).unwrap());
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
}
