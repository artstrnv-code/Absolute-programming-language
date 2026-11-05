# Tools

- `build_studio.ps1` builds the canonical Studio, CLI, and libraries under
  `dist/package/`. Files under Cargo's `target/` are internal cache.
- `build_libraries.ps1` compiles every canonical `libraries/*.apl` source into
  ignored `build/libraries/*.aplmod` files by default. Developer CLI and Studio
  builds discover this generated workspace directory automatically.
- `build_release.ps1` creates the complete `dist/package/` payload, portable
  Windows ZIP, and SHA-256 checksum.
- `build_installer.ps1` builds the Inno Setup installer from that payload.
- `clean_workspace.ps1` removes only ignored, reproducible `build/`, `target/`,
  and the temporary PowerShell GUI source.
- `legacy/` contains the old PowerShell GUI, batch wrappers, and one-off
  historical scripts. They must not be used as normal project tooling.
