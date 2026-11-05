# Tools

- `build_studio.ps1` builds the canonical Studio, CLI, and libraries under
  `dist/package/`. Its internal Cargo output lives under the ignored
  `build/studio-target/`, not the developer `target/release/` directory.
- `build_libraries.ps1` compiles every canonical `libraries/*.apl` source into
  ignored `build/libraries/*.aplmod` files by default. Developer CLI and Studio
  builds discover this generated workspace directory automatically. Module
  dependencies are resolved through a clean compiler staging directory beside
  the selected internal CLI instead of source-adjacent or previously generated
  artifacts.
- `build_release.ps1` creates the complete `dist/package/` payload, portable
  Windows ZIP, and SHA-256 checksum, after verifying the staged payload.
- `build_installer.ps1` builds the Inno Setup installer from that payload.
- `verify_package.ps1` validates the portable tree and exercises its packaged
  CLI. Add `-NativeSmoke` to compile and run a standalone EXE through the
  packaged offline native runtime.
- `clean_workspace.ps1` removes only ignored, reproducible `build/`, `target/`,
  and the temporary PowerShell GUI source.
- `legacy/` contains the old PowerShell GUI, batch wrappers, and one-off
  historical scripts. They must not be used as normal project tooling.
