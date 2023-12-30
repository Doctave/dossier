use std::collections::VecDeque;
use std::path::PathBuf;
use std::time::Instant;

use dossier_core::DocsParser;

use glob::glob;

fn main() {
    let start = Instant::now();
    let mut files: VecDeque<String> = std::env::args().map(String::from).collect();
    // Remove binary name
    files.pop_front();

    let mut out = vec![];
    let mut processed_files = vec![];

    for file in files {
        for entry in glob(&file).expect("Failed to read glob pattern") {
            match entry {
                Ok(path) => {
                    let file = PathBuf::from(&path);
                    if file.is_dir() {
                        continue;
                    }
                    match file.extension().and_then(|s| s.to_str()) {
                        Some("ts") => {
                            let parser = dossier_ts::Parser {};

                            match parser.parse(&file, &mut dossier_core::Context::new()) {
                                Ok(mut entities) => {
                                    out.append(&mut entities);
                                    processed_files.push(file);
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
                Err(e) => println!("{:?}", e),
            }
        }
    }
    let duration = start.elapsed();

    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    eprintln!(
        "Processed {} files in {}",
        processed_files.len(),
        pretty_duration::pretty_duration(&duration, None)
    );
}
