//! Read-only override discovery. Never serialize variable values or execute shell files.
use serde::Serialize;
use std::{collections::BTreeSet, fs, io::Read, path::Path};

const VARIABLES: &[(&str, &str)] = &[
    ("ANTHROPIC_API_KEY", "Claude Code"),
    ("ANTHROPIC_AUTH_TOKEN", "Claude Code"),
    ("ANTHROPIC_BASE_URL", "Claude Code"),
    ("ANTHROPIC_MODEL", "Claude Code"),
    ("CLAUDE_CODE_OAUTH_TOKEN", "Claude Code"),
    ("OPENAI_API_KEY", "Codex"),
    ("OPENAI_BASE_URL", "Codex"),
];
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    pub variable: String,
    pub application: String,
    pub source: String,
    pub kind: &'static str,
}
#[derive(Debug, Serialize)]
pub struct Diagnostics {
    pub findings: Vec<Finding>,
    pub warnings: Vec<String>,
}

pub fn inspect() -> Diagnostics {
    inspect_at(
        &crate::config::get_home_dir(),
        std::env::vars_os()
            .map(|(key, value)| (key.to_string_lossy().into_owned(), !value.is_empty())),
    )
}
fn inspect_at(home: &Path, environment: impl IntoIterator<Item = (String, bool)>) -> Diagnostics {
    let mut result = Diagnostics {
        findings: vec![],
        warnings: vec![],
    };
    let vars: BTreeSet<_> = environment
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(key, _)| key)
        .collect();
    for (name, app) in VARIABLES {
        if vars.contains(*name) {
            result.findings.push(Finding {
                variable: (*name).into(),
                application: (*app).into(),
                source: "Server process environment".into(),
                kind: "process",
            });
        }
    }
    for file in [
        ".bashrc",
        ".bash_profile",
        ".zshrc",
        ".zprofile",
        ".profile",
    ] {
        let path = home.join(file);
        let read = (|| -> Result<Option<String>, String> {
            let meta = match fs::symlink_metadata(&path) {
                Ok(meta) => meta,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(_) => return Err("could not inspect".into()),
            };
            if !meta.is_file() {
                return Err("not a regular file; skipped".into());
            }
            let mut bytes = vec![];
            fs::File::open(&path)
                .map_err(|_| "could not read")?
                .take(1_048_577)
                .read_to_end(&mut bytes)
                .map_err(|_| "could not read")?;
            if bytes.len() > 1_048_576 {
                return Err("larger than 1 MiB; skipped".into());
            }
            String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| "not UTF-8; skipped".into())
        })();
        match read {
            Ok(Some(text)) => {
                let names: BTreeSet<_> = text.lines().filter_map(assignment_name).collect();
                for (name, app) in VARIABLES {
                    if names.contains(*name) {
                        result.findings.push(Finding {
                            variable: (*name).into(),
                            application: (*app).into(),
                            source: path.display().to_string(),
                            kind: "shell",
                        });
                    }
                }
            }
            Ok(None) => {}
            Err(error) => result.warnings.push(format!("{}: {error}", path.display())),
        }
    }
    result
}
fn assignment_name(line: &str) -> Option<&str> {
    let line = line.trim();
    let line = if let Some(rest) = line.strip_prefix("export") {
        if rest.starts_with(char::is_whitespace) {
            rest.trim_start()
        } else {
            line
        }
    } else {
        line
    };
    let (name, _) = line.split_once('=')?;
    if name.bytes().all(|c| c.is_ascii_uppercase() || c == b'_') && !name.is_empty() {
        Some(name)
    } else {
        None
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reports_names_and_sources_only_without_executing_or_writing_files() {
        let home = tempfile::tempdir().unwrap();
        let path = home.path().join(".bashrc");
        let source = "# OPENAI_API_KEY=comment\nexport ANTHROPIC_API_KEY='secret-value'\nexport OPENAI_BASE_URL=\"$(touch should-not-exist)\"\nexport ANTHROPIC_API_KEY=again\necho OPENAI_API_KEY=not-assignment\n";
        fs::write(&path, source).unwrap();
        let result = inspect_at(
            home.path(),
            [
                ("OPENAI_API_KEY".into(), true),
                ("ANTHROPIC_MODEL".into(), false),
            ],
        );
        assert_eq!(result.findings.len(), 3);
        assert!(result.warnings.is_empty());
        let json = serde_json::to_string(&result).unwrap();
        assert!(!json.contains("secret-value"));
        assert!(!json.contains("touch"));
        assert_eq!(fs::read_to_string(path).unwrap(), source);
        assert!(!home.path().join("should-not-exist").exists());
    }
    #[test]
    fn partial_scan_reports_invalid_files_instead_of_claiming_complete_success() {
        let home = tempfile::tempdir().unwrap();
        fs::create_dir(home.path().join(".bashrc")).unwrap();
        fs::write(home.path().join(".profile"), [255]).unwrap();
        let result = inspect_at(home.path(), []);
        assert_eq!(result.warnings.len(), 2);
        assert!(result.findings.is_empty());
    }
}
