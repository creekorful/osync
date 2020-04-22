# ftpsync

Tool to synchronize in a effective/optimized way a **LOT** of files to an FTP server.
Designed for one-to-one (simplex) communication.

# Usage

``python -m ftpsync [-h] [--skip-upload SKIP_UPLOAD] src dst``

# How does it work?

The first time you'll run ftpsync, it will generate a local cache of the assumed server state after upload is completed.
Next calls will use the cache to only upload new/changed files, and delete old ones.

This will largely decrease upload duration when there's a larger number of files involved. 