use crate::{
    cli::CommandLineOptions,
    phpstan::{Analysis, Analyzer, AnalyzerOptions, Issue},
    protocol::{read_message, write_message},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::path::Path;
use std::{
    collections::HashMap,
    env, fs,
    io::{self, BufRead, Write},
    path::PathBuf,
};
use url::Url;

/// Diagnostics-only LSP server; PHPStan remains the analysis engine.
pub struct Server {
    root: PathBuf,
    documents: HashMap<String, String>,
    analyzer: Analyzer,
    command_line_options: CommandLineOptions,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InitializationOptions {
    phpstan_path: Option<PathBuf>,
    phpstan_config_path: Option<PathBuf>,
    memory_limit: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum Method {
    Initialize,
    Initialized,
    Shutdown,
    Exit,
    DidOpen,
    DidChange,
    DidSave,
    DidClose,
    Unknown,
}

impl From<Option<&str>> for Method {
    fn from(method: Option<&str>) -> Self {
        match method {
            Some("initialize") => Self::Initialize,
            Some("initialized") => Self::Initialized,
            Some("shutdown") => Self::Shutdown,
            Some("exit") => Self::Exit,
            Some("textDocument/didOpen") => Self::DidOpen,
            Some("textDocument/didChange") => Self::DidChange,
            Some("textDocument/didSave") => Self::DidSave,
            Some("textDocument/didClose") => Self::DidClose,
            _ => Self::Unknown,
        }
    }
}

impl Server {
    pub fn new(command_line_options: CommandLineOptions) -> io::Result<Self> {
        let root = env::current_dir()?;
        Ok(Self {
            analyzer: Analyzer::new(AnalyzerOptions::new(root.clone())),
            root,
            documents: HashMap::new(),
            command_line_options,
        })
    }

    pub fn run(&mut self, mut input: impl BufRead, mut output: impl Write) -> io::Result<()> {
        while let Some(message) = read_message(&mut input)? {
            let method = Method::from(message.get("method").and_then(Value::as_str));
            let id = message.get("id").cloned();
            match method {
                Method::Initialize => {
                    self.configure(&message);
                    if let Some(id) = id {
                        respond(&mut output, id, initialize_result())?;
                    }
                }
                Method::Initialized => {}
                Method::Shutdown => {
                    if let Some(id) = id {
                        respond(&mut output, id, Value::Null)?;
                    }
                }
                Method::Exit => return Ok(()),
                Method::DidOpen | Method::DidChange => {
                    if let Some(uri) = update_document(&message, &mut self.documents) {
                        self.publish_diagnostics(&uri, &mut output)?;
                    }
                }
                Method::DidSave => {
                    if let Some(uri) = document_uri(&message) {
                        self.publish_diagnostics(uri, &mut output)?;
                    }
                }
                Method::DidClose => {
                    if let Some(uri) = document_uri(&message) {
                        self.documents.remove(uri);
                        publish(&mut output, uri, Vec::new())?;
                    }
                }
                Method::Unknown => {
                    if let Some(id) = id {
                        respond(&mut output, id, Value::Null)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn configure(&mut self, message: &Value) {
        let params = &message["params"];
        self.root = workspace_root(params).unwrap_or_else(|| self.root.clone());
        let options: InitializationOptions =
            serde_json::from_value(params["initializationOptions"].clone()).unwrap_or_default();
        self.analyzer.configure(AnalyzerOptions {
            workspace_root: self.root.clone(),
            executable_path: self
                .command_line_options
                .phpstan_path
                .clone()
                .or(options.phpstan_path),
            configuration_path: self
                .command_line_options
                .configuration_path
                .clone()
                .or(options.phpstan_config_path),
            memory_limit: self
                .command_line_options
                .memory_limit
                .clone()
                .or(options.memory_limit),
        });
    }

    fn publish_diagnostics(&self, uri: &str, output: &mut impl Write) -> io::Result<()> {
        let Some(file) = path_from_uri(uri) else {
            return publish_error(output, uri, "Only file:// document URIs are supported.");
        };
        let source = self
            .documents
            .get(uri)
            .cloned()
            .unwrap_or_else(|| fs::read_to_string(&file).unwrap_or_default());

        match self.analyzer.analyse(&file, &source) {
            Ok(analysis) => publish_analysis(output, uri, &source, analysis),
            Err(error) => publish_error(output, uri, &error),
        }
    }
}

fn initialize_result() -> Value {
    json!({
        "capabilities": {"textDocumentSync": {"openClose": true, "change": 1}},
        "serverInfo": {"name": "phpstan-diagnostics-lsp", "version": env!("CARGO_PKG_VERSION")},
    })
}

fn workspace_root(params: &Value) -> Option<PathBuf> {
    params["rootUri"]
        .as_str()
        .and_then(path_from_uri)
        .or_else(|| params["rootPath"].as_str().map(PathBuf::from))
        .or_else(|| {
            params["workspaceFolders"]
                .as_array()
                .and_then(|folders| folders.first())
                .and_then(|folder| folder["uri"].as_str())
                .and_then(path_from_uri)
        })
}

fn document_uri(message: &Value) -> Option<&str> {
    message["params"]["textDocument"]["uri"].as_str()
}

fn update_document(message: &Value, documents: &mut HashMap<String, String>) -> Option<String> {
    let uri = document_uri(message)?.to_owned();
    let text = message["params"]["textDocument"]["text"]
        .as_str()
        .or_else(|| {
            message["params"]["contentChanges"]
                .as_array()?
                .last()?
                .get("text")?
                .as_str()
        })?;
    documents.insert(uri.clone(), text.to_owned());
    Some(uri)
}

fn path_from_uri(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}

fn publish_analysis(
    output: &mut impl Write,
    uri: &str,
    source: &str,
    analysis: Analysis,
) -> io::Result<()> {
    let file = path_from_uri(uri);
    let mut diagnostics: Vec<_> = analysis
        .issues
        .into_iter()
        .filter(|issue| {
            file.as_deref()
                .is_some_and(|file| same_file(file, &issue.file))
        })
        .map(|issue| diagnostic_from_issue(issue, source))
        .collect();
    if diagnostics.is_empty() && !analysis.errors.is_empty() {
        let (start_character, end_character) = line_character_range(source, 0);
        diagnostics.push(diagnostic(
            0,
            start_character,
            end_character,
            analysis.errors.join("\n"),
            None,
        ));
    }
    publish(output, uri, diagnostics)
}

fn same_file(document: &Path, reported: &Path) -> bool {
    document == reported
        || matches!(
            (document.canonicalize(), reported.canonicalize()),
            (Ok(document), Ok(reported)) if document == reported
        )
}

fn diagnostic_from_issue(issue: Issue, source: &str) -> Value {
    let line = issue.line.saturating_sub(1);
    let (start_character, end_character) = line_character_range(source, line);
    diagnostic(
        line,
        start_character,
        end_character,
        issue.message,
        issue.identifier,
    )
}

fn line_character_range(source: &str, line: u64) -> (u64, u64) {
    let line = source
        .split('\n')
        .nth(line as usize)
        .unwrap_or_default()
        .trim_end_matches('\r');
    let leading_whitespace = &line[..line.len() - line.trim_start().len()];
    let content = line.trim();
    let start_character = leading_whitespace.encode_utf16().count() as u64;
    let end_character = start_character + content.encode_utf16().count() as u64;
    (start_character, end_character)
}

fn diagnostic(
    line: u64,
    start_character: u64,
    end_character: u64,
    message: String,
    code: Option<String>,
) -> Value {
    json!({
        "range": {"start": {"line": line, "character": start_character}, "end": {"line": line, "character": end_character}},
        "severity": 1,
        "source": "phpstan",
        "code": code,
        "message": message,
    })
}

fn respond(output: &mut impl Write, id: Value, result: Value) -> io::Result<()> {
    write_message(
        output,
        &json!({"jsonrpc": "2.0", "id": id, "result": result}),
    )
}

fn publish(output: &mut impl Write, uri: &str, diagnostics: Vec<Value>) -> io::Result<()> {
    write_message(
        output,
        &json!({
            "jsonrpc": "2.0",
            "method": "textDocument/publishDiagnostics",
            "params": {"uri": uri, "diagnostics": diagnostics},
        }),
    )
}

fn publish_error(output: &mut impl Write, uri: &str, message: &str) -> io::Result<()> {
    publish(
        output,
        uri,
        vec![diagnostic(0, 0, 1, message.to_owned(), None)],
    )
}

#[cfg(test)]
mod tests {
    use super::{
        InitializationOptions, Method, line_character_range, update_document, workspace_root,
    };
    use serde_json::json;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn takes_document_text_from_a_change_notification() {
        let message = json!({
            "params": {
                "textDocument": {"uri": "file:///project/Foo.php"},
                "contentChanges": [{"text": "<?php $value = 1;"}]
            }
        });
        let mut documents = HashMap::new();

        assert_eq!(
            update_document(&message, &mut documents).as_deref(),
            Some("file:///project/Foo.php")
        );
        assert_eq!(documents["file:///project/Foo.php"], "<?php $value = 1;");
    }

    #[test]
    fn prefers_root_uri_over_legacy_root_path() {
        let params = json!({"rootUri": "file:///workspace", "rootPath": "/ignored"});
        assert_eq!(workspace_root(&params), Some(PathBuf::from("/workspace")));
    }

    #[test]
    fn classifies_supported_and_unknown_methods() {
        assert_eq!(Method::from(Some("textDocument/didSave")), Method::DidSave);
        assert_eq!(Method::from(Some("workspace/symbol")), Method::Unknown);
        assert_eq!(Method::from(None), Method::Unknown);
    }

    #[test]
    fn deserializes_phpstan_initialization_options() {
        let options: InitializationOptions = serde_json::from_value(json!({
            "phpstanPath": "dev-script/phpstan",
            "phpstanConfigPath": "phpstan.neon.dist",
            "memoryLimit": "1G",
        }))
        .unwrap();

        assert_eq!(
            options.phpstan_path,
            Some(PathBuf::from("dev-script/phpstan"))
        );
        assert_eq!(
            options.phpstan_config_path,
            Some(PathBuf::from("phpstan.neon.dist"))
        );
        assert_eq!(options.memory_limit.as_deref(), Some("1G"));
    }

    #[test]
    fn calculates_the_non_whitespace_line_range_in_utf16_code_units() {
        assert_eq!(
            line_character_range("<?php\n  $value = 😀;  \r\n", 1),
            (2, 14)
        );
    }
}
