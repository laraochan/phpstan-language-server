# phpstan-language-server

A diagnostics-only [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) adapter for [PHPStan](https://phpstan.org/).

PHPStan does not implement LSP itself. This server receives document updates through stdio, invokes the workspace's PHPStan in [editor mode](https://phpstan.org/user-guide/editor-mode), and publishes PHPStan findings as LSP diagnostics. It deliberately does not implement completion, navigation, or formatting.

## Requirements

- PHPStan 1.12.27+ or 2.1.17+ in the workspace (`vendor/bin/phpstan` by default). These versions support PHPStan editor mode.
- A PHP runtime suitable for the project's Composer dependencies.

## Install

From a clone of this repository:

```sh
cargo install --path .
```

After publishing to crates.io, installation will be:

```sh
cargo install phpstan-language-server
```

The server uses standard input/output for LSP. Configure your editor to start
`phpstan-language-server` with the workspace as its working directory. It
discovers `vendor/bin/phpstan`; clients can instead set
`initializationOptions.phpstanPath` to an absolute path or a workspace-relative
path.

## Development

```sh
cargo run --release
```

Optional initialization options:

```json
{
  "phpstanPath": "tools/phpstan",
  "phpstanConfigPath": "phpstan.neon.dist",
  "memoryLimit": "1G"
}
```

`phpstanConfigPath` accepts an absolute path or one relative to the workspace.
When set, the server passes it to PHPStan as `--configuration`; this supports
`phpstan.neon`, `phpstan.neon.dist`, `phpstan.dist`, and other configuration
file names.

On `didOpen`, `didChange`, and `didSave`, the current buffer is written to a temporary file and PHPStan is invoked with `--tmp-file` and `--instead-of`. This ensures unsaved buffer content, not only the on-disk file, is analysed.
