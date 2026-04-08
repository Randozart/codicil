use std::path::PathBuf;
use std::process::Command;

use crate::context::{RequestContext, Response};
use crate::route_file::RouteFile;

#[derive(Debug)]
pub struct Handler {
    pub route_file: RouteFile,
    pub file_path: PathBuf,
}

impl Handler {
    pub fn new(route_file: RouteFile, file_path: PathBuf) -> Self {
        Self {
            route_file,
            file_path,
        }
    }

    pub async fn execute(&self, ctx: RequestContext) -> HandlerResult {
        let brief_code = &self.route_file.brief_code;

        if brief_code.trim().is_empty() {
            return Ok(Response::new(200, "OK"));
        }

        let temp_dir = std::env::temp_dir().join("codicil-routes");
        std::fs::create_dir_all(&temp_dir).map_err(|e| HandlerError::Io(e.to_string()))?;

        let temp_file = temp_dir.join(format!(
            "{}.{}.bv",
            self.route_file.method,
            self.route_file.path.replace("/", "_").replace(":", "_")
        ));

        std::fs::write(&temp_file, brief_code)
            .map_err(|e| HandlerError::Io(e.to_string()))?;

        let context_json = serde_json::json!({
            "method": ctx.method,
            "path": ctx.path,
            "params": ctx.params,
            "query": ctx.query,
            "headers": ctx.headers,
            "body": ctx.body,
            "user": ctx.user,
            "session": ctx.session,
        });

        let context_file = temp_dir.join("context.json");
        std::fs::write(&context_file, context_json.to_string())
            .map_err(|e| HandlerError::Io(e.to_string()))?;

        let brief_path = std::env::var("BRIEF_PATH")
            .unwrap_or_else(|_| "/home/randozart/Desktop/Projects/brief-compiler/target/release/brief-compiler".to_string());

        let output = Command::new(&brief_path)
            .arg("check")
            .arg(&temp_file)
            .output()
            .map_err(|e| HandlerError::BriefCompiler(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(HandlerError::CompilationFailed(format!(
                "stdout: {}\nstderr: {}",
                stdout, stderr
            )));
        }

        let response = Response::new(200, format!("Route: {} {}", self.route_file.method, self.route_file.path));

        Ok(response)
    }
}

#[derive(Debug)]
pub enum HandlerError {
    Io(String),
    BriefCompiler(String),
    CompilationFailed(String),
    PreconditionFailed(String),
    PostconditionFailed(String),
}

impl std::fmt::Display for HandlerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::Io(msg) => write!(f, "IO error: {}", msg),
            HandlerError::BriefCompiler(msg) => write!(f, "Brief compiler error: {}", msg),
            HandlerError::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            HandlerError::PreconditionFailed(msg) => write!(f, "Precondition failed: {}", msg),
            HandlerError::PostconditionFailed(msg) => write!(f, "Postcondition failed: {}", msg),
        }
    }
}

impl std::error::Error for HandlerError {}

pub type HandlerResult = Result<Response, HandlerError>;
