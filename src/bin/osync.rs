use std::process;

use clap::{crate_authors, crate_version, App, Arg};

use osync::index::Index;

fn main() {
    let matches = App::new("osync")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Synchronize efficientlyc LOT of files to remote server")
        .arg(
            Arg::with_name("src")
                .value_name("SRC")
                .help("Sets a custom config file"),
        )
        .arg(
            Arg::with_name("dst")
                .value_name("DST")
                .help("Sets the input file to use"),
        )
        .get_matches();

    let src = matches.value_of("src").unwrap();
    let dst = matches.values_of("dst").unwrap();

    // Read previous index (if any)
    let previous_index = match Index::load(src) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("error while reading index: {}", e);
            process::exit(1);
        }
    };
    println!("Index of {} files loaded", previous_index.len());

    // Compute current index
    let current_index = match Index::compute(src) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("error while computing index: {}", e);
            process::exit(1);
        }
    };
    println!("Index of {} files computed", current_index.len());
}
