use serde::Deserialize;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub struct Analyzer {
    root: PathBuf,
    phpstan_path: Option<PathBuf>,
    config_path: Option<PathBuf>,
    memory_limit: Option<String>,
}

pub struct Analysis {
    pub issues: Vec<Issue>,
    pub errors: Vec<String>,
}

pub struct Issue {
    pub file: PathBuf,
    pub line: u64,
    pub message: String,
    pub identifier: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PhpstanOutput {
    #[serde(default)]
    files: HashMap<String, PhpstanFile>,
    #[serde(default)]
    errors: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PhpstanFile {
    #[serde(default)]
    messages: Vec<PhpstanMessage>,
}

#[derive(Debug, Deserialize)]
struct PhpstanMessage {
    message: String,
    #[serde(default = "default_line_number")]
    line: u64,
    identifier: Option<String>,
}

fn default_line_number() -> u64 {
    1
}

impl Analyzer {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            phpstan_path: None,
            config_path: None,
            memory_limit: None,
        }
    }

    pub fn configure(
        &mut self,
        root: PathBuf,
        phpstan_path: Option<PathBuf>,
        config_path: Option<PathBuf>,
        memory_limit: Option<String>,
    ) {
        self.root = root;
        self.phpstan_path = phpstan_path;
        self.config_path = config_path;
        self.memory_limit = memory_limit;
    }

    pub fn analyse(&self, file: &Path, source: &str) -> Result<Analysis, String> {
        let temporary = temporary_file(source).map_err(|error| error.to_string())?;
        let result = self.run(file, &temporary);
        let _ = fs::remove_file(temporary);
        result
    }

    fn run(&self, file: &Path, temporary: &Path) -> Result<Analysis, String> {
        let binary = self.find_binary().ok_or_else(|| {
            format!(
                "PHPStan was not found. Install phpstan/phpstan in this workspace or set initializationOptions.phpstanPath. Looked for {}.",
                self.root.join("vendor/bin/phpstan").display()
            )
        })?;
        let temporary = temporary.to_string_lossy();
        let file = file.to_string_lossy();
        let mut command = Command::new(binary);
        command.current_dir(&self.root).args([
            "analyse",
            "--error-format=json",
            "--no-progress",
            "--no-ansi",
            "--tmp-file",
            &temporary,
            "--instead-of",
            &file,
            &file,
        ]);
        if let Some(config) = self.configuration_path() {
            command.arg("--configuration").arg(config);
        }
        if let Some(limit) = &self.memory_limit {
            command.arg(format!("--memory-limit={limit}"));
        }

        let output = command
            .output()
            .map_err(|error| format!("Could not start PHPStan: {error}"))?;
        parse_output(&output.stdout).map_err(|error| {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            if stderr.is_empty() {
                format!("PHPStan returned invalid JSON: {error}")
            } else {
                format!("PHPStan failed: {stderr}")
            }
        })
    }

    fn find_binary(&self) -> Option<PathBuf> {
        let configured = self.phpstan_path.as_ref().map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                self.root.join(path)
            }
        });
        configured.filter(|path| path.is_file()).or_else(|| {
            ["vendor/bin/phpstan", "vendor/bin/phpstan.bat"]
                .into_iter()
                .map(|candidate| self.root.join(candidate))
                .find(|candidate| candidate.is_file())
        })
    }

    fn configuration_path(&self) -> Option<PathBuf> {
        self.config_path.as_ref().map(|path| {
            if path.is_absolute() {
                path.clone()
            } else {
                self.root.join(path)
            }
        })
    }
}

fn parse_output(stdout: &[u8]) -> Result<Analysis, serde_json::Error> {
    let output: PhpstanOutput = serde_json::from_slice(stdout)?;
    let issues = output
        .files
        .into_iter()
        .flat_map(|(path, file)| {
            file.messages.into_iter().map(move |message| Issue {
                file: PathBuf::from(&path),
                line: message.line,
                message: message.message,
                identifier: message.identifier,
            })
        })
        .collect();
    Ok(Analysis {
        issues,
        errors: output.errors,
    })
}

fn temporary_file(contents: &str) -> std::io::Result<PathBuf> {
    let name = format!(
        "phpstan-language-server-{}-{}.php",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let path = env::temp_dir().join(name);
    fs::write(&path, contents)?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::{Analyzer, parse_output};
    use std::path::PathBuf;

    #[test]
    fn parses_file_messages_and_global_errors() {
        let output = br#"{
            "files": {"/project/Foo.php": {"messages": [{
                "message": "Undefined variable: $value",
                "line": 12,
                "identifier": "variable.undefined"
            }]}},
            "errors": ["Configuration file is invalid"]
        }"#;

        let analysis = parse_output(output).unwrap();
        assert_eq!(analysis.issues.len(), 1);
        assert_eq!(analysis.issues[0].file, PathBuf::from("/project/Foo.php"));
        assert_eq!(analysis.issues[0].line, 12);
        assert_eq!(
            analysis.issues[0].identifier.as_deref(),
            Some("variable.undefined")
        );
        assert_eq!(analysis.errors, ["Configuration file is invalid"]);
    }

    #[test]
    fn resolves_workspace_relative_configuration_file() {
        let mut analyzer = Analyzer::new(PathBuf::from("/workspace"));
        analyzer.configure(
            PathBuf::from("/workspace"),
            None,
            Some(PathBuf::from("phpstan.neon.dist")),
            None,
        );

        assert_eq!(
            analyzer.configuration_path(),
            Some(PathBuf::from("/workspace/phpstan.neon.dist"))
        );
    }
}
