# phpstan-diagnostics-lsp

> [!WARNING]
> This is an experimental prototype, built primarily for personal use. Its
> configuration and behavior may change without notice.

A diagnostics-only [Language Server Protocol](https://microsoft.github.io/language-server-protocol/) adapter for [PHPStan](https://phpstan.org/).

PHPStan does not implement LSP itself. This server receives document updates through stdio, invokes the workspace's PHPStan in [editor mode](https://phpstan.org/user-guide/editor-mode), and publishes PHPStan findings as LSP diagnostics. It deliberately does not implement completion, navigation, or formatting.

## Motivation

I wanted PHPStan diagnostics in editors such as Helix without using VS Code or
running Phpactor. This project adapts a workspace's PHPStan CLI output to the
Language Server Protocol.

## Scope

The server currently publishes diagnostics for opened PHP files only. It does
not provide completion, navigation, formatting, or project-wide background
diagnostics. It is intended for personal use and experimentation, not
production use.

## Requirements

- PHPStan 1.12.27+ or 2.1.17+ in the workspace (`vendor/bin/phpstan` by default). These versions support PHPStan editor mode.
- A PHP runtime suitable for the project's Composer dependencies.

## Install from source

From a clone of this repository:

```sh
cargo install --path .
```

The server uses standard input/output for LSP. Configure your editor to start
`phpstan-diagnostics-lsp` with the workspace as its working directory. It
discovers `vendor/bin/phpstan`; clients can instead set
`initializationOptions.phpstanPath` to an absolute path or a workspace-relative
path.

## Usage

```sh
phpstan-diagnostics-lsp [OPTIONS]
```

For example, configure the PHPStan executable and configuration file when the
server is started:

```sh
phpstan-diagnostics-lsp \
  --phpstan-path dev-script/phpstan \
  --configuration phpstan.neon.dist \
  --memory-limit 1G
```

Available options:

- `--phpstan-path <PATH>` chooses the PHPStan executable. By default, the
  server uses `vendor/bin/phpstan` in the workspace.
- `-c`, `--configuration <PATH>` chooses the PHPStan configuration file. It
  accepts an absolute path or a path relative to the workspace.
- `--memory-limit <LIMIT>` passes a PHP memory limit such as `1G` to PHPStan.
- `-h`, `--help` prints the available options.

This is a stdio language server and is normally started by an editor LSP
configuration. The same settings can be passed as LSP initialization options
(`phpstanPath`, `phpstanConfigPath`, and `memoryLimit`), but explicit CLI
options take precedence.

`--configuration` supports `phpstan.neon`, `phpstan.neon.dist`,
`phpstan.dist`, and other configuration file names.

On `didOpen`, `didChange`, and `didSave`, the current buffer is written to a
temporary file and PHPStan is invoked with `--tmp-file` and `--instead-of`.
This ensures unsaved buffer content, not only the on-disk file, is analysed.

## Feedback

Bug reports, configuration examples, and small improvement ideas are welcome.
Please open an issue with your editor, PHPStan version, configuration, and a
minimal reproduction where possible. As this is a personal prototype, response
times and implementation are not guaranteed.

## License

This project is licensed under the [MIT License](LICENSE).
