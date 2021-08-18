
Dates are specified in YYYY-MM-DD format.

# Unreleased

## Changed
- Fix failing to delete files on some freedesktop (eg Linux) systems when the home was not mounted at the root.
- Fix for test failing on Linux environments that don't have a desktop environment (more specifically don't have a tool like `gio`)

# v2.0.1 on 2021-05-02

## Changed
- Fix not being able to trash any item on some systems.

# v2.0.0 on 2021-04-20

## Changed
- The "Linux" implementation was replaced by a custom Freedesktop implementation.

## Added
- `list`, `purge_all`, and `restore_all` to Windows and Freedesktop
