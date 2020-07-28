use std::process;

use clap::{crate_authors, crate_version, App, Arg};
use url::Url;

use osync::index::Index;
use osync::sync::{FtpSync, Sync};

fn main() {
    let matches = App::new("osync")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Synchronize efficiently LOT of files to remote server")
        .arg(Arg::with_name("src").value_name("SRC"))
        .arg(Arg::with_name("dst").value_name("DST"))
        .get_matches();

    let src = matches.value_of("src").unwrap();
    let dst = match matches.value_of("dst").map(|v| Url::parse(v)).unwrap() {
        Ok(url) => url,
        Err(e) => {
            eprintln!("invalid src: {}", e);
            process::exit(1);
        }
    };

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

    // Synchronize the files
    let synchronizer = FtpSync {};
    match synchronizer.synchronize(&previous_index, &current_index, &dst) {
        Ok(_) => println!("todo"),
        Err(e) => {
            eprintln!("error while synchronizing files: {}", e);
            process::exit(1);
        }
    }
}
