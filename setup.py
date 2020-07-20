import setuptools

with open("README.md", "r") as fh:
    long_description = fh.read()

setuptools.setup(
    name="ftpsync-creekorful",
    version="1.1.0",
    author="Aloïs Micard",
    author_email="alois@micard.lu",
    description="Tool to synchronize in a effective/optimized way a lot of files to an FTP server.",
    long_description=long_description,
    long_description_content_type="text/markdown",
    url="https://github.com/creekorful/ftpsync",
    packages=setuptools.find_packages(),
    classifiers=[
        "Programming Language :: Python :: 3",
        "License :: OSI Approved :: MIT License",
        "Operating System :: OS Independent",
    ],
    python_requires='>=3.6',
)
