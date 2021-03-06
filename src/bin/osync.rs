use std::process;

use clap::{crate_authors, crate_version, App, AppSettings, Arg};
use url::Url;

use osync::index::Index;
use osync::sync::{FtpSync, Sync};

fn main() {
    let matches = App::new("osync")
        .version(crate_version!())
        .author(crate_authors!())
        .about("Synchronize efficiently LOT of files to FTP server")
        .arg(
            Arg::with_name("src")
                .value_name("SRC")
                .required(true)
                .help("The source directory."),
        )
        .arg(
            Arg::with_name("dst")
                .value_name("DST")
                .help("The destination. (f.e: ftp://user:pass@ftp.example.org/test-folder)"),
        )
        .arg(
            Arg::with_name("assume-directories")
                .long("assume-directories")
                .help("Use the local index to determinate existing directories"),
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches();

    let src = matches.value_of("src").unwrap();
    let dst = matches.value_of("dst").map(|v| Url::parse(v).unwrap());
    let assume_directories = matches.is_present("assume-directories");

    // Read previous index (if any)
    let mut previous_index = match Index::load(src) {
        Ok(index) => index,
        Err(e) => {
            eprintln!("error while reading index: {}", e);
            process::exit(1);
        }
    };
    println!("Index of {} files loaded", previous_index.len());

    // Compute current index
    let current_index = match Index::compute(src) {
        Ok((index, ignored_files)) => {
            println!("({} files ignored)", ignored_files);
            index
        }
        Err(e) => {
            eprintln!("error while computing index: {}", e);
            process::exit(1);
        }
    };
    println!("Index of {} files computed", current_index.len());

    // Synchronize the files
    let mut synchronizer = match FtpSync::new(&dst) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error while connecting to the server: {}", e);
            process::exit(1);
        }
    };

    match synchronizer.synchronize(&current_index, &mut previous_index, assume_directories) {
        Ok(skip_upload) => {
            if skip_upload {
                println!("Synchronization successful! (upload skipped)")
            } else {
                println!("Synchronization successful!")
            }
        }
        Err(e) => {
            eprintln!("error while synchronizing files: {}", e);
            process::exit(1);
        }
    }
}
