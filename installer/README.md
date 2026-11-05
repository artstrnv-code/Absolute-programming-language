# Installer Sources

`windows/apl.iss` defines the current per-user Windows installer. It consumes
the exact payload generated under `dist/package/` by `tools/build_release.ps1`.

Build it through `tools/build_installer.ps1`; do not invoke the Inno compiler
against an arbitrary directory, because the release script also compiles and
verifies the shipped `.aplmod` files.
