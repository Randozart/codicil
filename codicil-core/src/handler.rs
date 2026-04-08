use std::path::PathBuf;
use std::process::Command;

use crate::context::{ApiError, RequestContext, Response};
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
            .arg("build")
            .arg(&temp_file)
            .output()
            .map_err(|e| HandlerError::BriefCompiler(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !output.status.success() {
            return Err(HandlerError::CompilationFailed(format!(
                "stdout: {}\nstderr: {}",
                stdout, stderr
            )));
        }

        let response = Self::parse_response_from_output(&stdout, self.route_file.method.clone(), self.route_file.path.clone())?;

        Ok(response)
    }

    fn parse_response_from_output(output: &str, method: String, path: String) -> HandlerResult {
        if output.trim().is_empty() {
            return Ok(Response::new(200, format!("Route: {} {}", method, path)));
        }

        if let Ok(json) = serde_json::from_str::<serde_json::Value>(output) {
            let status = json.get("status")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16)
                .unwrap_or(200);

            let body = json.get("body")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let mut response = Response::new(status, body);

            if let Some(headers) = json.get("headers").and_then(|h| h.as_object()) {
                for (k, v) in headers {
                    if let Some(v_str) = v.as_str() {
                        response.headers.insert(k.clone(), v_str.to_string());
                    }
                }
            }

            return Ok(response);
        }

        Ok(Response::new(200, output.to_string()))
    }
}

pub struct ErrorHandler {
    pub error_route_path: PathBuf,
}

impl ErrorHandler {
    pub fn new(error_route_path: PathBuf) -> Self {
        Self { error_route_path }
    }

    pub async fn execute(&self, error: HandlerError, ctx: RequestContext) -> HandlerResult {
        let route_file = match RouteFile::parse(&self.error_route_path) {
            Ok(rf) => rf,
            Err(e) => {
                return Ok(self.default_error_response(error));
            }
        };

        let brief_code = &route_file.brief_code;

        if brief_code.trim().is_empty() {
            return Ok(self.default_error_response(error));
        }

        let temp_dir = std::env::temp_dir().join("codicil-routes");
        std::fs::create_dir_all(&temp_dir).map_err(|e| HandlerError::Io(e.to_string()))?;

        let temp_file = temp_dir.join("error.bv");
        std::fs::write(&temp_file, brief_code)
            .map_err(|e| HandlerError::Io(e.to_string()))?;

        let api_error = match &error {
            HandlerError::PreconditionFailed(msg) => {
                ApiError::bad_request(msg.clone())
            }
            HandlerError::PostconditionFailed(msg) => {
                ApiError::internal_error(msg.clone())
            }
            HandlerError::CompilationFailed(msg) => {
                ApiError::internal_error(format!("Compilation failed: {}", msg))
            }
            HandlerError::BriefCompiler(msg) => {
                ApiError::internal_error(format!("Brief compiler error: {}", msg))
            }
            HandlerError::Io(msg) => {
                ApiError::internal_error(format!("IO error: {}", msg))
            }
        };

        let context_json = serde_json::json!({
            "method": ctx.method,
            "path": ctx.path,
            "params": ctx.params,
            "query": ctx.query,
            "headers": ctx.headers,
            "body": ctx.body,
            "user": ctx.user,
            "session": ctx.session,
            "error": {
                "code": api_error.code,
                "message": api_error.message,
                "details": api_error.details,
            },
        });

        let context_file = temp_dir.join("error_context.json");
        std::fs::write(&context_file, context_json.to_string())
            .map_err(|e| HandlerError::Io(e.to_string()))?;

        let brief_path = std::env::var("BRIEF_PATH")
            .unwrap_or_else(|_| "/home/randozart/Desktop/Projects/brief-compiler/target/release/brief-compiler".to_string());

        let output = Command::new(&brief_path)
            .arg("build")
            .arg(&temp_file)
            .output()
            .map_err(|e| HandlerError::BriefCompiler(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();

        if output.status.success() && !stdout.trim().is_empty() {
            return Handler::parse_response_from_output(&stdout, "ERROR".to_string(), "/[error]".to_string());
        }

        Ok(self.default_error_response(error))
    }

    fn default_error_response(&self, error: HandlerError) -> Response {
        let api_error = match error {
            HandlerError::PreconditionFailed(msg) => {
                ApiError::bad_request(msg)
            }
            HandlerError::PostconditionFailed(msg) => {
                ApiError::internal_error(msg)
            }
            HandlerError::CompilationFailed(msg) => {
                ApiError::internal_error(format!("Compilation failed: {}", msg))
            }
            HandlerError::BriefCompiler(msg) => {
                ApiError::internal_error(format!("Brief compiler error: {}", msg))
            }
            HandlerError::Io(msg) => {
                ApiError::internal_error(format!("IO error: {}", msg))
            }
        };

        api_error.to_response(500)
    }
}

#[derive(Debug, Clone)]
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
