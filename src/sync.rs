use std::collections::HashMap;
use std::path::Path;

use ftp::FtpStream;

use crate::index::Index;

pub trait Sync {
    fn synchronize<P: AsRef<Path>>(
        &self,
        path: P,
        a: &Index,
        b: &Index,
        config: &HashMap<String, String>,
    );
}

/// A synchronizer which save by FTP.
pub struct FtpSync {}

impl Sync for FtpSync {
    fn synchronize<P: AsRef<Path>>(
        &self,
        path: P,
        a: &Index,
        b: &Index,
        config: &HashMap<String, String>,
    ) {
        unimplemented!()
    }
}
