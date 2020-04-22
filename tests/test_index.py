import os

from ftpsync.index import compute_file_checksum


def test_compute_file_checksum():
    with open('.test-file', 'w+') as f:
        f.write('test')
    assert compute_file_checksum('.test-file') == '9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08'
    os.unlink('.test-file')
