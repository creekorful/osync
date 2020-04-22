"""
Contain logic related to index
"""

import hashlib
from pathlib import Path

INDEX_FILE = '.ftpsync'


def compute_file_checksum(file_path: str) -> str:
    """
    Compute the SHA-256 checksum of given file
    :param file_path: path to the file to compute checksum for
    :return: computed file checksum
    """
    hasher = hashlib.sha256()

    with open(file_path, 'rb') as file:
        while True:
            chunk = file.read(hasher.block_size)
            if not chunk:
                break
            hasher.update(chunk)

    return hasher.hexdigest()


def load_index(file_path: str) -> dict:
    """
    Load the checksums from given index file
    :param file_path: path to the index file
    :return: loaded checksums
    """
    checksums = {}
    with open(file_path, 'r') as file:
        for line in file:
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
    with open(file_path, 'w+') as file:
        for entry, checksum in checksums.items():
            file.write("{}:{}\n".format(entry, checksum))


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
