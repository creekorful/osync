"""
Module with ftp related helper functions
"""

import ftplib
import os
from urllib.parse import ParseResult


class FtpSession:
    """ Represent an FTP session """

    def __init__(self, host: ParseResult):
        """
        Instantiate the ftp session using given host
        :param host: host details
        """
        self.files_cache = {}
        self.ftp = ftplib.FTP()

        port = 21 if host.port is None else host.port
        self.ftp.connect(host.hostname, port, 5)
        self.ftp.login(host.username, host.password)

        if host.path is not None:
            self.ftp.cwd(host.path)

    def __del__(self):
        """
        Close the ftp connection upon destruction
        """
        self.ftp.quit()

    def directory_exists(self, haystack: str, needle: str) -> bool:
        """
        Determinate if given directory needle exist in haystack parent directory
        :param haystack: directory to find
        :param needle: directory to look into
        :return: true if needle is find in haystack
        """

        if os.path.join(haystack, needle) in self.files_cache:
            return True

        for file, facts in self.ftp.mlsd(haystack, facts=['type']):
            if file == needle and facts['type'] == 'dir':
                self.files_cache[os.path.join(haystack, needle)] = True
                return True
        return False

    def make_directories(self, path: str):
        """
        Make any missing directories that well be needed to allow storage on given path
        :param path: the path
        """
        current_dir = ""

        for directory in os.path.split(path)[0].split(os.sep):
            next_dir = os.path.join(current_dir, directory)
            if not self.directory_exists(current_dir, directory):
                self.ftp.mkd(next_dir)
                current_dir = next_dir

    def delete(self, file):
        """
        Delete given file
        :param file: file path
        """
        self.ftp.delete(file)

    def upload(self, path: str, content):
        """
        Upload given content to given path
        :param path: remote path where to store the content
        :param content: content to be uploaded
        """
        self.ftp.storbinary("STOR {}".format(path), content)
