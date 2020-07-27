use clap::{crate_authors, crate_version, App, Arg};

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
}
