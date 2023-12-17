use std::collections::VecDeque;
use std::path::PathBuf;

fn main() {
    let mut files: VecDeque<PathBuf> = std::env::args().map(PathBuf::from).collect();
    // Remove binary name
    files.pop_front();

    for file in files {
        match file.extension().and_then(|s| s.to_str()) {
            Some("ts") => match dossier_ts::Parser::parse(file) {
                Ok(_) => {}
                Err(e) => {
                    eprint!("Error parsing docs");
                    std::process::exit(1);
                }
            },
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
