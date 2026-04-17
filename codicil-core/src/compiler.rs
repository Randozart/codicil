use std::path::{Path, PathBuf};
use std::process::Command;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompilerError {
    #[error("Brief compiler not found at: {0}")]
    NotFound(PathBuf),
    #[error("Compilation failed: {0}")]
    CompilationFailed(String),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

pub struct BriefCompiler {
    path: PathBuf,
}

impl BriefCompiler {
    pub fn new() -> Result<Self, CompilerError> {
        let path = Self::find_brief()?;
        Ok(Self { path })
    }

    pub fn with_path(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    fn find_brief() -> Result<PathBuf, CompilerError> {
        // Check BRIEF_PATH environment variable first
        if let Ok(path) = std::env::var("BRIEF_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
            eprintln!(
                "Warning: BRIEF_PATH set to {:?} but file does not exist",
                path
            );
        }

        // Check local build
        let local_path = PathBuf::from("target/release/brief-compiler");
        if local_path.exists() {
            return Ok(local_path);
        }

        // Check various common installation paths
        let search_paths = vec![
            PathBuf::from("/usr/local/bin/brief"),
            PathBuf::from("/usr/bin/brief"),
            PathBuf::from("$HOME/.cargo/bin/brief"),
            PathBuf::from("$HOME/.local/bin/brief"),
        ];

        for path in search_paths {
            let expanded = path
                .to_str()
                .unwrap_or("")
                .replace("$HOME", &std::env::var("HOME").unwrap_or_default());
            let expanded = PathBuf::from(expanded);
            if expanded.exists() {
                return Ok(expanded);
            }
        }

        // Try using 'which' command to find brief
        if let Ok(output) = Command::new("which").arg("brief").output() {
            if output.status.success() {
                let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !path.is_empty() && PathBuf::from(&path).exists() {
                    return Ok(PathBuf::from(path));
                }
            }
        }

        // Try using 'brief' directly (might be in PATH)
        if Command::new("brief")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(PathBuf::from("brief"));
        }

        Err(CompilerError::NotFound(PathBuf::from("brief")))
    }

    pub fn check(&self, file: &Path) -> Result<CheckResult, CompilerError> {
        let output = Command::new(&self.path).arg("check").arg(file).output()?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CheckResult {
            success,
            stdout,
            stderr,
        })
    }

    pub fn build(&self, file: &Path) -> Result<BuildResult, CompilerError> {
        let output = Command::new(&self.path).arg("build").arg(file).output()?;

        let success = output.status.success();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(BuildResult {
            success,
            stdout,
            stderr,
        })
    }
}

impl Default for BriefCompiler {
    fn default() -> Self {
        Self::new().expect("Brief compiler not found")
    }
}

#[derive(Debug)]
pub struct CheckResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub struct BuildResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_path_detection() {
        let compiler = BriefCompiler::new();
        match compiler {
            Ok(c) => println!("Found Brief compiler at: {:?}", c.path),
            Err(e) => println!("Brief compiler not found: {}", e),
        }
    }
}
