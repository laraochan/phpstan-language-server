# phpstan-language-server

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
`phpstan-language-server` with the workspace as its working directory. It
discovers `vendor/bin/phpstan`; clients can instead set
`initializationOptions.phpstanPath` to an absolute path or a workspace-relative
path.

## Configuration

The exact configuration syntax is editor-specific. This server accepts the
following optional LSP initialization options:

- `phpstanPath` chooses the PHPStan executable. By default, the server uses
  `vendor/bin/phpstan` in the workspace.
- `phpstanConfigPath` chooses the PHPStan configuration file. It accepts an
  absolute path or a path relative to the workspace, and is passed to PHPStan
  as `--configuration`.
- `memoryLimit` passes a PHP memory limit such as `1G` to PHPStan.

`phpstanConfigPath` supports `phpstan.neon`, `phpstan.neon.dist`,
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
