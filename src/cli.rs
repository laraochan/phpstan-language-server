use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommandLineOptions {
    pub phpstan_path: Option<PathBuf>,
    pub configuration_path: Option<PathBuf>,
    pub memory_limit: Option<String>,
}

#[derive(Debug)]
pub enum CommandLineAction {
    Start(CommandLineOptions),
    Help,
}

pub const HELP: &str = r#"Usage: phpstan-diagnostics-lsp [OPTIONS]

A diagnostics-only Language Server Protocol adapter for PHPStan.

Options:
  --phpstan-path <PATH>     Path to the PHPStan executable
  -c, --configuration <PATH>
                            Path to the PHPStan configuration file
  --memory-limit <LIMIT>    PHP memory limit passed to PHPStan (for example, 1G)
  -h, --help                Print help
"#;

pub fn parse(arguments: impl IntoIterator<Item = String>) -> Result<CommandLineAction, String> {
    let mut options = CommandLineOptions::default();
    let mut arguments = arguments.into_iter();

    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "-h" | "--help" => return Ok(CommandLineAction::Help),
            "--phpstan-path" => {
                options.phpstan_path = Some(PathBuf::from(next_value(&mut arguments, &argument)?));
            }
            "-c" | "--configuration" => {
                options.configuration_path =
                    Some(PathBuf::from(next_value(&mut arguments, &argument)?));
            }
            "--memory-limit" => {
                options.memory_limit = Some(next_value(&mut arguments, &argument)?);
            }
            _ => {
                return Err(format!(
                    "Unknown argument: {argument}\n\nRun phpstan-diagnostics-lsp --help for usage."
                ));
            }
        }
    }

    Ok(CommandLineAction::Start(options))
}

fn next_value(
    arguments: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<String, String> {
    arguments.next().ok_or_else(|| {
        format!("Missing value for {option}\n\nRun phpstan-diagnostics-lsp --help for usage.")
    })
}

#[cfg(test)]
mod tests {
    use super::{CommandLineAction, CommandLineOptions, parse};
    use std::path::PathBuf;

    #[test]
    fn parses_phpstan_options() {
        let action = parse([
            "--phpstan-path".to_owned(),
            "dev-script/phpstan".to_owned(),
            "-c".to_owned(),
            "phpstan.neon.dist".to_owned(),
            "--memory-limit".to_owned(),
            "1G".to_owned(),
        ])
        .unwrap();

        let CommandLineAction::Start(options) = action else {
            panic!("expected start action");
        };
        assert_eq!(
            options,
            CommandLineOptions {
                phpstan_path: Some(PathBuf::from("dev-script/phpstan")),
                configuration_path: Some(PathBuf::from("phpstan.neon.dist")),
                memory_limit: Some("1G".to_owned()),
            }
        );
    }

    #[test]
    fn parses_help() {
        assert!(matches!(
            parse(["--help".to_owned()]),
            Ok(CommandLineAction::Help)
        ));
    }

    #[test]
    fn rejects_unknown_arguments() {
        assert!(
            parse(["--unknown".to_owned()])
                .unwrap_err()
                .contains("Unknown argument")
        );
    }
}
