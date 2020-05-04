"""
ftpsync is the main module
containing core logic
"""
import logging
import os
import sys
from urllib.parse import urlparse

import click

from ftpsync import ftp as ftputil
from ftpsync import index


def process_changed_files(ftp: ftputil.FtpSession, files: list, src_path: str):
    """
    Process the changed files (new/changed)
    :param ftp: the opened FTP client
    :param files: files to be uploaded
    :param src_path: local src path (if any)
    """
    current_files = 0
    total_files = len(files)
    for file in files:
        current_files += 1

        # make any missing directories (if any)
        if os.sep in file:
            ftp.make_directories(file)

        logging.debug("Uploading file %s (%d/%d)",
                      file, current_files, total_files)

        # Upload the file
        ftp.upload(file, open("{}{}".format(src_path, file), 'rb'))


def process_deleted_files(ftp: ftputil.FtpSession, files: list):
    """
    Process the deleted files
    :param ftp: the opened FTP client
    :param files: files to be deleted
    """
    current_files = 0
    total_files = len(files)
    for file in files:
        current_files += 1

        logging.debug("Deleting file %s (%d/%d)",
                      file, current_files, total_files)

        ftp.delete(file)


@click.command()
@click.argument('src')
@click.argument('dst', required=False)
@click.option('--skip-sync', is_flag=True, flag_value=True, help='do not synchronize files, only generate index')
@click.version_option('1.1.0')
def main(src, dst, skip_sync):
    """Synchronize efficiently LOT of files to FTP server."""

    src = "{}/".format(src) if not src.endswith('/') else src

    # Configure logging
    logging.basicConfig(format='%(asctime)s - %(levelname)s - %(message)s', level=logging.DEBUG)

    # Create index file if needed
    index_file_path = os.path.join(src, index.INDEX_FILE)
    if not os.path.exists(index_file_path):
        logging.debug("%s file not found in source directory. Creating one.", index.INDEX_FILE)
        open(index_file_path, 'w').close()

    # Load previous index
    previous_index = index.load_index(index_file_path)
    logging.debug("Checksum of %d files loaded.", len(previous_index))

    # Compute current index
    current_index = index.compute_index(src)
    logging.debug("Checksum of %d files computed.", len(current_index))

    # If we only want to generate index, skip the sync
    if skip_sync:
        logging.info("Generating index file for %s (skipping synchronization)", src)
        index.save_index(index_file_path, current_index)
        sys.exit(0)

    if dst is None:
        logging.error('Argument \'DST\' is required.')
        exit(1)

    dst = urlparse("ftp://{}".format(dst) if not dst.startswith('ftp://') else dst)

    logging.info("Synchronizing %s to %s%s", src, dst.hostname, dst.path)

    # Compute the difference
    changed_files, deleted_files = index.diff_index(previous_index, current_index)
    logging.info("%d files have changed", len(changed_files))
    logging.info("%d files have been deleted", len(deleted_files))

    if len(changed_files) == 0 and len(deleted_files) == 0:
        logging.info('Nothing has changed')
        sys.exit(0)

    # Connect to the FTP server
    try:
        ftp_session = ftputil.FtpSession(dst)
    except TimeoutError as err:
        logging.error("Error while connecting to the FTP server: %s", err)
        sys.exit(1)

    # First of all upload changed files (new/changed)
    process_changed_files(ftp_session, changed_files, src)

    # Then delete deleted files
    process_deleted_files(ftp_session, deleted_files)

    # Save current index
    index.save_index(index_file_path, current_index)


if __name__ == '__main__':
    main(prog_name='ftpsync.py')
