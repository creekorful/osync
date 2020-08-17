# osync

![Crates.io](https://img.shields.io/crates/v/osync)
[![osync](https://snapcraft.io//osync/badge.svg)](https://snapcraft.io/osync)

Tool to synchronize in a optimized way a **LOT** of files to a FTP server.

## Why

One day I neded to upload >500,000 files to a remote FTP server (with lftp).
it was slow and tedious (as expected considering the number of files).

Sadly further uploads were really slow too. The mirroring mode of lftp
wasn't really helpful considering the huge amount of directories / files involved.

That's why I have developed osync (previously ftpsync):
to help user uplodad huge amount of files in a convenient way.

## How

Osync use a local cache to assume the server state. Each time you upload something,
it will read the cache to determinate which file has changed / has been deleted, etc...
this is **WAY** faster than trying to determinate on the server wich file should be uploaded.

Please note that this software **SET** the server state using current state. If you have done upload on a computer 'A',
and run the script from a computer 'B', everything will be replaced to make the server looks like 'B'.

This could be a problem depending on your use case.  

## How to install

You can install the latest version of osync using cargo

```sh
cargo install osync
```

Or with [snap](https://snapcraft.io).

```sh
snap install osync
```