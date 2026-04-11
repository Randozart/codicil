use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::context::RequestContext;
use crate::route_file::RouteFile;

#[derive(Error, Debug)]
pub enum MiddlewareError {
    #[error("Failed to read middleware file: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Middleware not found: {0}")]
    NotFound(String),
    #[error("Middleware execution failed: {0}")]
    ExecutionFailed(String),
}

#[derive(Debug, Clone)]
pub struct Middleware {
    pub name: String,
    pub file_path: PathBuf,
    pub route_file: RouteFile,
}

impl Middleware {
    pub fn load(name: &str, project_root: &Path) -> Result<Self, MiddlewareError> {
        let middleware_path = project_root.join("middleware").join(format!("{}.bv", name));

        if !middleware_path.exists() {
            return Err(MiddlewareError::NotFound(name.to_string()));
        }

        let route_file = RouteFile::parse(&middleware_path)
            .map_err(|e| MiddlewareError::IoError(std::io::Error::other(e.to_string())))?;

        Ok(Self {
            name: name.to_string(),
            file_path: middleware_path,
            route_file,
        })
    }

    pub async fn execute(&self, _ctx: &mut RequestContext) -> Result<(), MiddlewareError> {
        tracing::info!("Executing middleware: {}", self.name);
        Ok(())
    }
}

#[derive(Debug)]
pub struct MiddlewareChain {
    middleware: Vec<Middleware>,
}

impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middleware: Vec::new(),
        }
    }

    pub fn from_names(names: &[String], project_root: &Path) -> Result<Self, MiddlewareError> {
        let mut chain = Self::new();

        for name in names {
            let mw = Middleware::load(name, project_root)?;
            chain.middleware.push(mw);
        }

        Ok(chain)
    }

    pub async fn execute(
        &self,
        mut ctx: RequestContext,
    ) -> Result<RequestContext, MiddlewareError> {
        for mw in &self.middleware {
            mw.execute(&mut ctx).await?;
        }
        Ok(ctx)
    }

    pub fn is_empty(&self) -> bool {
        self.middleware.is_empty()
    }
}

impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}

pub struct MiddlewareBuilder {
    middleware_names: Vec<String>,
}

impl MiddlewareBuilder {
    pub fn new() -> Self {
        Self {
            middleware_names: Vec::new(),
        }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn add(mut self, name: &str) -> Self {
        self.middleware_names.push(name.to_string());
        self
    }

    pub fn build(self) -> Vec<String> {
        self.middleware_names
    }
}

impl Default for MiddlewareBuilder {
    fn default() -> Self {
        Self::new()
    }
}
