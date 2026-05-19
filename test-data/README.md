# This directory is scratch area for all tests

Tests would create and measure size of files inside this dir. 
Tests won't touch files outside it.

## Subdirs
### `long-path`

This dir isn't checked into the GIT. It's files would be automatically
created by tests. It would test if dirstat can handle really long file path.

It's not checked into the git so it won't be in repo causing confusion,
and GIT client on Windows can have troubles checking it out. It requires
special option in git config configured globally, and it can cause compatibility issues. 

### `pre-created`

This dir contains pre created files of known size.
Files are inside git, to allow them having unchanging compressibility, so
tests that check compressed files would have predictable results.

Please be advised not to add new-lines to text files, as newline can be checked out differently on Unix and Windows,
causing files to have different size, which would mess with tests.
