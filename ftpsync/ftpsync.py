import argparse
import ftplib
import hashlib
import logging
import os
from pathlib import Path
from urllib.parse import urlparse

INDEX_FILE = '.ftpsync'


def compute_file_checksum(file_path: str) -> str:
    """
    Compute the SHA-256 checksum of given file
    :param file_path: path to the file to compute checksum for
    :return: computed file checksum
    """
    h = hashlib.sha256()

    with open(file_path, 'rb') as f:
        while True:
            chunk = f.read(h.block_size)
            if not chunk:
                break
            h.update(chunk)

    return h.hexdigest()


def load_index(file_path: str) -> dict:
    """
    Load the checksums from given index file
    :param file_path: path to the index file
    :return: loaded checksums
    """
    checksums = {}
    with open(file_path, 'r') as f:
        for line in f:
            clean_line = line.rstrip()
            part = clean_line.split(":")
            checksums[part[0]] = part[1]
    return checksums


def save_index(file_path: str, checksums: dict):
    """
    Save given index file
    :param file_path: path to the index file
    :param checksums: checksums to be saved
    """
    with open(file_path, 'w+') as f:
        for file, checksum in checksums.items():
            f.write("{}:{}\n".format(file, checksum))


def compute_index(directory: str) -> dict:
    """
    Compute index for given directory
    :param directory: directory to compute index for
    :return: computed index
    """
    checksums = {}
    for entry in Path(directory).rglob("*"):
        if entry.is_file() and INDEX_FILE not in entry.as_posix():
            checksums[entry.as_posix().lstrip(directory)] = compute_file_checksum(entry.as_posix())
    return checksums


def diff_index(previous: dict, current: dict) -> (dict, dict):
    """
    Compute the difference between previous & current index
    :param previous: the previous index (last state)
    :param current: the current index (new state)
    :return: tuple, with (new/changed) entries first, deleted entries second
    """
    changed = []
    deleted = []
    for entry, value in current.items():
        if entry not in previous or previous.get(entry) != value:
            changed.append(entry)

    for entry in previous:
        if entry not in current:
            deleted.append(entry)

    return changed, deleted


def ftp_directory_exists(ftp: ftplib.FTP, haystack: str, needle: str) -> bool:
    """
    Determinate if given needle directory exist in dir directory
    :param ftp: the opened FTP client
    :param haystack: dir to look needle for
    :param needle: dir to find
    :return: true if needle exist in dir. False otherwise
    """
    for file, facts in ftp.mlsd(haystack, facts=['type']):
        if file == needle and facts['type'] == 'dir':
            return True
    return False


def ftp_make_directories(ftp: ftplib.FTP, path: str):
    """
    Create the directories needed to allow storage in given file path
    This method will create all missing directories recursively to allow storage in given file path
    :param ftp: the opened FTP client
    :param path: the path to be created (excluding file if any)
    """
    current_dir = ""

    for directory in os.path.split(path)[0].split(os.sep):
        next_dir = os.path.join(current_dir, directory)
        if not ftp_directory_exists(ftp, current_dir, directory):
            logging.debug("Making missing directory: {}".format(next_dir))
            ftp.mkd(next_dir)
        current_dir = next_dir


if __name__ == '__main__':
    # Configure parser
    parser = argparse.ArgumentParser(description='Synchronize efficiently source directory to destination directory')
    parser.add_argument('src', type=str, help='the source directory to be synchronized')
    parser.add_argument('dst', type=str, help='the destination directory where files should be synchronized')
    parser.add_argument('--init', dest='init_only', type=bool,
                        help='only generate index file, do not synchronize files')
    args = parser.parse_args()

    # Bind variables
    src_path = "{}/".format(args.src) if not args.src.endswith('/') else args.src
    dst = urlparse("ftp://{}".format(args.dst) if not args.dst.startswith('ftp://') else args.dst)

    # Configure logging
    logging.basicConfig(format='%(asctime)s - %(levelname)s - %(message)s', level=logging.DEBUG)

    if not args.init_only:
        logging.info("Synchronizing {} to {}/".format(src_path, dst.hostname, dst.path))
    else:
        logging.info("Generating index file for {} (Skipping upload)".format(src_path))

    # Create index file if needed
    index_file_path = os.path.join(src_path, INDEX_FILE)
    if not os.path.exists(index_file_path):
        logging.debug("{} file not found in source directory. Creating one.".format(INDEX_FILE))
        logging.info("This is the first upload of {} to {}{}".format(src_path, dst.hostname, dst.path))
        open(index_file_path, 'w').close()

    # Load previous index
    previous_index = load_index(index_file_path)
    logging.debug("Checksum of {} files loaded.".format(len(previous_index)))

    # Compute current index
    current_index = compute_index(src_path)
    logging.debug("Checksum of {} files computed.".format(len(current_index)))

    # If we only want to generate index, skip the rest
    if args.init_only:
        save_index(index_file_path, current_index)
        exit(0)

    # Compute the difference
    changed_files, deleted_files = diff_index(previous_index, current_index)
    logging.info("{} files have changed".format(len(changed_files)))
    logging.info("{} files have been deleted".format(len(deleted_files)))

    if len(changed_files) == 0 and len(deleted_files) == 0:
        logging.info('Nothing has changed')
        exit(0)

    # Connect to the FTP server
    ftp_session = ftplib.FTP()
    port = 21 if dst.port is None else dst.port
    try:
        ftp_session.connect(dst.hostname, port, timeout=5)
        ftp_session.login(dst.username, dst.password)
    except Exception as err:
        logging.error("Error while connecting to the FTP server: {}".format(err))
        exit(1)

    # Go to target directory
    ftp_session.cwd(dst.path)

    # First of all upload changed files (new/changed)
    current_changed_files = 0
    total_changed_files = len(changed_files)
    for changed_file in changed_files:
        current_changed_files += 1

        # make any missing directories (if any)
        if os.sep in changed_file:
            ftp_make_directories(ftp_session, changed_file)

        logging.debug("Uploading file {} ({}/{})".format(changed_file, current_changed_files, total_changed_files))

        # Upload the file
        ftp_session.storbinary("STOR {}".format(changed_file), open("{}{}".format(src_path, changed_file), 'rb'))

    # Then delete deleted files
    current_deleted_files = 0
    total_deleted_files = len(deleted_files)
    for deleted_file in deleted_files:
        current_deleted_files += 1

        logging.debug("Deleting file {} ({}/{})".format(deleted_file, current_deleted_files, total_deleted_files))

        ftp_session.delete(deleted_file)

    ftp_session.quit()

    # Save current index
    save_index(index_file_path, current_index)
