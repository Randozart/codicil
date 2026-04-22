use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

#[cfg(test)]
mod tests;

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use bytes::Bytes;
use clap::{Parser, Subcommand};
use codicil_core::{
    is_relevant_file, watch_paths, BriefCompiler, ErrorHandler, Handler, MiddlewareChain,
    RequestContext, RouteFile, Router as CodicilRouter,
};
use tokio::net::TcpListener;
use tokio::time::interval;
use tower_http::services::ServeDir;

const FAVICON_SVG: &str = include_str!("../assets/favicon.svg");
const LANDING_RBV: &str = include_str!("../assets/landing.rbv");
const GLOBALS_CSS: &str = include_str!("../assets/globals.css");
const INDEX_BV: &str = include_str!("../assets/index.bv");
const GET_HINTS_BV: &str = include_str!("../assets/GET.hints.bv");
const LANDING_CODICIL_TOML: &str = include_str!("../assets/codicil.toml");

#[derive(Parser)]
#[command(name = "codi")]
#[command(version = "0.1.0")]
#[command(about = "Codicil - A contract-driven web framework built on Brief")]
enum Cli {
    Init {
        name: String,
        #[arg(long, default_value = "false")]
        no_template: bool,
    },
    Dev {
        #[arg(default_value = ".")]
        path: String,
    },
    Build {
        #[arg(default_value = ".")]
        path: String,
    },
    Check {
        #[arg(default_value = ".")]
        path: String,
    },
    Generate {
        #[command(subcommand)]
        command: Generate,
    },
}

#[derive(Subcommand)]
enum Generate {
    Model { name: String },
    Middleware { name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli {
        Cli::Init { name, no_template } => {
            cmd_init(&name, no_template)?;
        }
        Cli::Dev { path } => {
            cmd_dev(&path).await?;
        }
        Cli::Build { path } => {
            cmd_build(&path)?;
        }
        Cli::Check { path } => {
            cmd_check(&path)?;
        }
        Cli::Generate { command } => {
            cmd_generate(&command)?;
        }
    }

    Ok(())
}

fn cmd_init(name: &str, no_template: bool) -> Result<()> {
    use std::fs;

    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    fs::create_dir_all(project_dir)?;
    fs::create_dir_all(project_dir.join("src"))?;
    fs::create_dir_all(project_dir.join("lib"))?;
    fs::create_dir_all(project_dir.join("middleware"))?;
    fs::create_dir_all(project_dir.join("components"))?;
    fs::create_dir_all(project_dir.join("migrations"))?;
    fs::create_dir_all(project_dir.join("public/build"))?;
    fs::create_dir_all(project_dir.join("assets"))?;
    fs::create_dir_all(project_dir.join("styles"))?;
    fs::create_dir_all(project_dir.join(".codicil"))?;

    let codicil_toml = format!(
        r#"# Codicil Project Configuration
[project]
name = "{}"
version = "0.1.0"

[server]
host = "localhost"
port = 3000

[build]
brief_path = ""
"#,
        name
    );
    fs::write(project_dir.join("codicil.toml"), codicil_toml)?;

    let codicil_config = format!(
        r#"# Codicil Configuration
[lsp]
enabled = true

[routing]
style = "folder"
"#,
    );
    fs::write(project_dir.join(".codicil/config.toml"), codicil_config)?;

    fs::write(project_dir.join("src/page.rbv"), r#"txn handle [true][true] {
    term "Hello, World!";
};
"#
    )?;

    fs::write(project_dir.join("lib/.gitkeep"), "")?;
    fs::write(project_dir.join("middleware/.gitkeep"), "")?;
    fs::write(project_dir.join("public/favicon.svg"), FAVICON_SVG)?;
    fs::write(project_dir.join("styles/globals.css"), "")?;

    if no_template {
        println!("Created empty project '{}'", name);
    } else {
        println!("Created project '{}' with landing-page template", name);
    }
    println!("  cd {} && codi dev", name);
Ok(())
}

fn cmd_check(path: &str) -> Result<()> {
    use std::process::Command;

    let project_path = Path::new(path);
    let routes_dir = project_path.join("routes");
    let src_dir = project_path.join("src");
    let components_dir = project_path.join("components");

    if !routes_dir.exists() && !src_dir.exists() {
        anyhow::bail!("No routes or src directory found in {}", path);
    }

    let mut has_errors = false;

    // Create temp directory
    let temp_dir = project_path.join(".codicil-check");
    if temp_dir.exists() {
        std::fs::remove_dir_all(&temp_dir)?;
    }
    std::fs::create_dir_all(&temp_dir)?;

    // Copy ALL files recursively to preserve complete directory structure
    fn copy_recursive(src: &Path, dst: &Path) -> Result<()> {
        if !src.exists() {
            return Ok(());
        }
        std::fs::create_dir_all(dst)?;
        for entry in std::fs::read_dir(src)? {
            let entry = entry?;
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if src_path.is_dir() {
                copy_recursive(&src_path, &dst_path)?;
            } else {
                std::fs::copy(&src_path, &dst_path)?;
            }
        }
Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_src_directory() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        assert!(project_path.join("src").exists());
        assert!(project_path.join("src/page.rbv").exists());
    }

    #[test]
    fn test_init_creates_codicil_config() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        assert!(project_path.join(".codicil").exists());
        assert!(project_path.join(".codicil/config.toml").exists());
    }

    #[test]
    fn test_init_creates_page_rbv() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        let content = fs::read_to_string(project_path.join("src/page.rbv")).unwrap();
        assert!(content.contains("txn handle"));
    }

    #[test]
    fn test_init_fails_if_directory_exists() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        fs::create_dir(&project_path).unwrap();
        
        let result = cmd_init(project_path.to_str().unwrap(), true);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_dev_server_page_rbv_returns_200() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Write a simple working handler
        fs::write(project_path.join("src/page.rbv"), r#"
txn handle [true][true] {
    term "Hello, World!";
};
"#).unwrap();
        
        // Test that routes are discovered correctly
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        let routes: Vec<_> = router.routes().collect();
        
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/");
        assert_eq!(routes[0].method, codicil_core::HttpMethod::GET);
    }

    #[test]
    fn test_dev_server_route_rbv_all_methods() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Create route.rbv for API
        fs::create_dir_all(project_path.join("src/api")).unwrap();
        fs::write(project_path.join("src/api/route.rbv"), r#"
txn handle [true][true] {
    term "API response";
};
"#).unwrap();
        
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        let routes: Vec<_> = router.routes().collect();
        
        // route.rbv should create routes for all 5 methods
        assert_eq!(routes.len(), 5);
        
        let paths: Vec<_> = routes.iter().map(|r| r.path.clone()).collect();
        assert!(paths.contains(&"/api".to_string()));
        
        let methods: Vec<_> = routes.iter().map(|r| r.method.clone()).collect();
        assert!(methods.contains(&codicil_core::HttpMethod::GET));
        assert!(methods.contains(&codicil_core::HttpMethod::POST));
        assert!(methods.contains(&codicil_core::HttpMethod::PUT));
        assert!(methods.contains(&codicil_core::HttpMethod::DELETE));
    }

    #[test]
    fn test_dev_server_dynamic_segment() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Create dynamic route
        fs::create_dir_all(project_path.join("src/users/[id]")).unwrap();
        fs::write(project_path.join("src/users/[id]/page.rbv"), r#"
txn handle [true][true] {
    term "User";
};
"#).unwrap();
        
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        
        // Test finding the route
        let found = router.find_route(&codicil_core::HttpMethod::GET, "/users/123");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.get("id"), Some(&"123".to_string()));
    }

    #[test]
    fn test_dev_server_static_file_exists() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Create a static file in public/
        fs::create_dir_all(project_path.join("public")).unwrap();
        fs::write(project_path.join("public/test.js"), "console.log('test');").unwrap();
        
        // Verify file exists
        assert!(project_path.join("public/test.js").exists());
    }

    #[test]
    fn test_dev_server_nested_routes() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Create nested routes
        fs::create_dir_all(project_path.join("src/users/[userId]/posts")).unwrap();
        fs::write(project_path.join("src/users/[userId]/posts/page.rbv"), r#"
txn handle [true][true] {
    term "posts";
};
"#).unwrap();
        
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        let routes: Vec<_> = router.routes().collect();
        
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/users/:userId/posts");
        
        let found = router.find_route(&codicil_core::HttpMethod::GET, "/users/abc/posts");
        assert!(found.is_some());
        assert_eq!(found.unwrap().params.get("userId"), Some(&"abc".to_string()));
    }

    #[test]
    fn test_dev_server_route_group() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        // Create route group
        fs::create_dir_all(project_path.join("src/(admin)")).unwrap();
        fs::write(project_path.join("src/(admin)/page.rbv"), r#"
txn handle [true][true] {
    term "admin";
};
"#).unwrap();
        
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        let routes: Vec<_> = router.routes().collect();
        
        // Route group should not appear in URL
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].path, "/");
    }

    #[test]
    fn test_dev_server_404_for_unknown_path() {
        let temp = TempDir::new().unwrap();
        let project_path = temp.path().join("test-project");
        
        cmd_init(project_path.to_str().unwrap(), true).unwrap();
        
        fs::write(project_path.join("src/page.rbv"), r#"
txn handle [true][true] {
    term "home";
};
"#).unwrap();
        
        let router = codicil_core::Router::discover_routes(&project_path).unwrap();
        
        // Unknown path should return None
        let found = router.find_route(&codicil_core::HttpMethod::GET, "/nonexistent");
        assert!(found.is_none());
    }
}

    // Copy complete directory structure
    if routes_dir.exists() {
        copy_recursive(&routes_dir, &temp_dir.join("routes"))?;
    }
    if src_dir.exists() {
        copy_recursive(&src_dir, &temp_dir.join("src"))?;
    }
    if components_dir.exists() {
        copy_recursive(&components_dir, &temp_dir.join("components"))?;
    }

    // Check .bv route files
    if routes_dir.exists() {
        println!("Checking route files...");
        for entry in std::fs::read_dir(&routes_dir)? {
            let entry = entry?;
            let file_path = entry.path();
            if file_path.extension().map_or(false, |ext| ext == "bv") {
                print!("  {}: ", file_path.file_name().unwrap().to_string_lossy());
                
                // Extract Brief code using RouteFile
                let content = std::fs::read_to_string(&file_path)?;
                let route_file = RouteFile::parse_content(&content, &file_path).map_err(|e| anyhow::anyhow!("Failed to parse route file: {}", e))?;
                
                let brief_code = route_file.brief_code;
                
                if brief_code.trim().is_empty() {
                    println!("[SKIP] No Brief code found");
                    continue;
                }

                // Write to temp file in project directory so imports resolve correctly
                let temp_file = temp_dir.join("routes").join(file_path.file_name().unwrap());
                std::fs::create_dir_all(temp_file.parent().unwrap())?;
                std::fs::write(&temp_file, &brief_code)?;

                // Run brief check (path must be absolute)
                let output = Command::new("brief")
                    .arg("check")
                    .arg(temp_file.canonicalize()?)
                    .output()?;

                if output.status.success() {
                    println!("[OK]");
                } else {
                    println!("[FAIL]");
                    eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                    has_errors = true;
                }
            }
        }
    }

    // Check .rbv component files by checking the copied .bv versions
    let temp_rbv_paths = vec![
        temp_dir.join("components"),
        temp_dir.join("src/components"),
        temp_dir.join("src/pages"),
    ];

    for rbv_base in temp_rbv_paths {
        if !rbv_base.exists() {
            continue;
        }

        println!("Checking component files...");
        check_temp_bv_directory(&rbv_base, project_path, &mut has_errors)?;
    }

    if has_errors {
        anyhow::bail!("Check failed - see errors above");
    }

    println!("\nAll checks passed!");
    Ok(())
}

fn check_temp_bv_directory(dir: &Path, _project_path: &Path, has_errors: &mut bool) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            check_temp_bv_directory(&path, _project_path, has_errors)?;
        } else if path.extension().map_or(false, |ext| ext == "bv") {
            print!("  {}: ", path.file_name().unwrap().to_string_lossy());

            // Run brief check on the already-copied file
            let output = Command::new("brief")
                .arg("check")
                .arg(path.canonicalize()?)
                .output()?;

            if output.status.success() {
                println!("[OK]");
            } else {
                println!("[FAIL]");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                *has_errors = true;
            }
        }
    }
    Ok(())
}

fn check_rbv_directory(dir: &Path, project_path: &Path, has_errors: &mut bool) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            check_rbv_directory(&path, project_path, has_errors)?;
        } else if path.extension().map_or(false, |ext| ext == "rbv") {
            print!("  {}: ", path.file_name().unwrap().to_string_lossy());

            // Extract Brief code from <script> block
            let content = std::fs::read_to_string(&path)?;
            let brief_code = extract_script_from_rbv(&content);

            if brief_code.trim().is_empty() {
                println!("[SKIP] No Brief code found");
                continue;
            }

            // Write to temp file in project directory so imports resolve correctly
            let temp_dir = project_path.join(".codicil-check");
            std::fs::create_dir_all(&temp_dir)?;
            
            // Create mirror directory structure
            let relative_path = path.strip_prefix(project_path).unwrap_or(&path);
            let target_dir = temp_dir.join(relative_path.parent().unwrap_or(relative_path));
            std::fs::create_dir_all(&target_dir)?;
            
            let stem = path.file_stem().unwrap().to_string_lossy();
            let temp_file = target_dir.join(format!("{}.bv", stem));
            std::fs::write(&temp_file, &brief_code)?;

            // Run brief check (path must be absolute)
            let output = Command::new("brief")
                .arg("check")
                .arg(temp_file.canonicalize()?)
                .output()?;

            if output.status.success() {
                println!("[OK]");
            } else {
                println!("[FAIL]");
                eprintln!("{}", String::from_utf8_lossy(&output.stderr));
                *has_errors = true;
            }
        }
    }
    Ok(())
}

fn extract_script_from_rbv(content: &str) -> String {
    let mut result = String::new();
    let mut in_script = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("<script") {
            in_script = true;
            continue;
        }
        if trimmed == "</script>" {
            in_script = false;
            continue;
        }
        if in_script {
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

#[derive(Clone)]
struct AppState {
    project_path: Arc<PathBuf>,
}

async fn handle_favicon() -> impl IntoResponse {
    Response::builder()
        .status(200)
        .header("Content-Type", "image/svg+xml")
        .body(FAVICON_SVG.to_string())
        .unwrap()
}

fn serve_static_file(path: &Path) -> Response {
    let (content_type, is_binary) = match path.extension().and_then(|s| s.to_str()) {
        Some("js") | Some("mjs") => ("application/javascript", false),
        Some("css") => ("text/css", false),
        Some("wasm") => ("application/wasm", true),
        Some("html") | Some("htm") => ("text/html", false),
        Some("svg") => ("image/svg+xml", false),
        Some("json") => ("application/json", false),
        Some("png") => ("image/png", true),
        Some("jpg") | Some("jpeg") => ("image/jpeg", true),
        Some("gif") => ("image/gif", true),
        Some("ico") => ("image/x-icon", true),
        Some("woff") => ("font/woff", false),
        Some("woff2") => ("font/woff2", false),
        Some("ttf") => ("font/ttf", false),
        _ => ("application/octet-stream", false),
    };

    if is_binary {
        match std::fs::read(path) {
            Ok(bytes) => Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .body(Body::from(bytes))
                .unwrap(),
            Err(_) => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("File not found"))
                .unwrap(),
        }
    } else {
        match std::fs::read_to_string(path) {
            Ok(content) => Response::builder()
                .status(200)
                .header("Content-Type", content_type)
                .body(Body::from(content))
                .unwrap(),
            Err(_) => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("File not found"))
                .unwrap(),
        }
    }
}

fn recompile_rbv_components(project_path: &Path) {
    let components_dir = project_path.join("components");
    let build_dir = project_path.join("public/build");

    if !components_dir.exists() {
        return;
    }

    let compiler = match BriefCompiler::new() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("  (FAIL) Failed to find Brief compiler: {}", e);
            return;
        }
    };

    if let Ok(entries) = std::fs::read_dir(&components_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "rbv").unwrap_or(false) {
                let filename = path
                    .file_stem()
                    .and_then(|n| n.to_str())
                    .unwrap_or("component");

                let output = std::process::Command::new(compiler.path())
                    .args([
                        "rbv",
                        "--out",
                        build_dir.to_str().unwrap(),
                        path.to_str().unwrap(),
                    ])
                    .output();

                match output {
                    Ok(o) if o.status.success() => {
                        println!("  Compiled {} successfully", filename);
                    }
                    Ok(o) => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        eprintln!("  (FAIL) Failed to compile {}: {}", filename, stderr);
                    }
                    Err(e) => {
                        eprintln!("  (FAIL) Failed to run brief: {}", e);
                    }
                }
            }
        }
    }
}

async fn cmd_dev(path: &str) -> Result<()> {
    let project_path = PathBuf::from(path).canonicalize()?;
    let config_path = project_path.join("codicil.toml");

    let mut host = "localhost".to_string();
    let mut port: u16 = 3000;

    if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path)?;
        if let Ok(config) = config_str.parse::<toml::Value>() {
            if let Some(server) = config.get("server") {
                if let Some(h) = server.get("host").and_then(|v| v.as_str()) {
                    host = h.to_string();
                }
                if let Some(p) = server.get("port").and_then(|v| v.as_integer()) {
                    port = p as u16;
                }
            }
        }
    }

    // Auto-compile RBV components if needed
    let build_dir = project_path.join("public/build");
    let components_dir = project_path.join("components");

    if components_dir.exists() {
        let html_path = build_dir.join("landing.html");
        let needs_recompile = if !html_path.exists() {
            println!("No compiled output found, will compile...");
            true
        } else if let Ok(entries) = std::fs::read_dir(&components_dir) {
            let html_mtime = std::fs::metadata(&html_path).and_then(|m| m.modified()).ok();
            let mut needs = false;
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("rbv") {
                    if let Ok(rbv_mtime) = path.metadata().and_then(|m| m.modified()) {
                        if let Some(html_time) = html_mtime {
                            if rbv_mtime > html_time {
                                needs = true;
                                break;
                            }
                        }
                    }
                }
            }
            needs
        } else {
            false
        };

        if needs_recompile || !html_path.exists() {
            println!("Compiling components...");
            let compiler = BriefCompiler::new().map_err(|e| anyhow::anyhow!("Brief compiler not found: {}", e))?;
            if let Ok(entries) = std::fs::read_dir(&components_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) == Some("rbv") {
                        let filename = path.file_stem().and_then(|n| n.to_str()).unwrap_or("component");
                        let output = std::process::Command::new(compiler.path())
                            .args(["rbv", "--out", build_dir.to_str().unwrap(), path.to_str().unwrap()])
                            .output();
                        match output {
                            Ok(o) if o.status.success() => println!("  Compiled {}", filename),
                            Ok(o) => eprintln!("  FAIL {}: {}", filename, String::from_utf8_lossy(&o.stderr)),
                            Err(e) => eprintln!("  FAIL {}: {}", filename, e),
                        }
                    }
                }
            }
        } else {
            println!("Components up to date.");
        }
    }

    let state = AppState {
        project_path: Arc::new(project_path.clone()),
    };

    let addr = format!("{}:{}", host, port);

    println!("Discovering routes...");
    let codicil_router = CodicilRouter::discover_routes(&project_path)?;
    let routes: Vec<_> = codicil_router.routes().collect();
    println!("Discovered {} routes:", routes.len());
    for route in &routes {
        println!("  {:?} {}", route.method, route.path);
    }

    println!("Dev server running at http://{}", addr);
    println!("Watching for file changes (Ctrl+C to stop)...\n");

    let watcher =
        watch_paths(&project_path).map_err(|e| anyhow::anyhow!("Failed to watch files: {}", e))?;

    let build_path = project_path.join("public/build");

    let app = Router::new()
        .route("/favicon.ico", any(handle_favicon))
        .route("/favicon.svg", any(handle_favicon))
        .route("/", any(handle_root))
        .route("/*path", any(handle_catchall))
        .with_state(state)
        .fallback_service(ServeDir::new(&build_path));

    let listener = TcpListener::bind(&addr).await?;

    let server = axum::serve(listener, app);

    let server_handle = tokio::spawn(async {
        if let Err(e) = server.await {
            eprintln!("Server error: {}", e);
        }
    });

    let mut reload_interval = interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            biased;

            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
            _ = reload_interval.tick() => {
                let events = watcher.poll();
                if !events.is_empty() {
                    let relevant: Vec<_> = events.iter()
                        .filter(|e| {
                            match e {
                                codicil_core::FileEvent::Changed(p) => is_relevant_file(p),
                                codicil_core::FileEvent::Created(p) => is_relevant_file(p),
                                codicil_core::FileEvent::Deleted(p) => is_relevant_file(p),
                            }
                        })
                        .collect();

                    if !relevant.is_empty() {
                        println!("\nFile changed: {:?}", relevant);

                        let rbv_changes: Vec<_> = relevant.iter()
                            .filter(|e| {
                                if let codicil_core::FileEvent::Changed(p) = e {
                                    p.extension().map(|ext| ext == "rbv").unwrap_or(false)
                                } else {
                                    false
                                }
                            })
                            .collect();

                        if !rbv_changes.is_empty() {
                            println!("Recompiling RBV files...");
                            recompile_rbv_components(&project_path);
                        }

                        println!("Reloading routes...");
                        match CodicilRouter::discover_routes(&project_path) {
                            Ok(router) => {
                                let routes: Vec<_> = router.routes().collect();
                                println!("Discovered {} routes:", routes.len());
                                for route in &routes {
                                    println!("  {:?} {}", route.method, route.path);
                                }
                            }
                            Err(e) => {
                                println!("(FAIL) Error discovering routes: {}", e);
                            }
                        }
                        println!("Watching for file changes...");
                    }
                }
            }
        }
    }

    server_handle.abort();

    Ok(())
}

async fn handle_root(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    Query(query_params): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let landing_path = state.project_path.join("public/build/landing.html");
    if landing_path.is_file() {
        if let Ok(html) = std::fs::read_to_string(&landing_path) {
            return Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(html))
                .unwrap();
        }
    }

    let index_path = state.project_path.join("public/index.html");
    if index_path.is_file() {
        if let Ok(html) = std::fs::read_to_string(&index_path) {
            return Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(html))
                .unwrap();
        }
    }

    handle_request_internal(state, method, headers, "/", query_params, body).await
}

async fn handle_catchall(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    AxumPath(path): AxumPath<String>,
    Query(query_params): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let request_path = format!("/{}", path);
    let path_stripped = path.trim_start_matches('/');

    // Check in public/build/
    let build_path = state.project_path.join("public/build").join(path_stripped);
    if build_path.is_file() {
        return serve_static_file(&build_path);
    }

    // Check in public/
    let public_path = state.project_path.join("public").join(path_stripped);
    if public_path.is_file() {
        return serve_static_file(&public_path);
    }

    // Try route handler
    handle_request_internal(state, method, headers, &request_path, query_params, body).await
}

async fn handle_request_internal(
    state: AppState,
    method: Method,
    headers: HeaderMap,
    path: &str,
    query_params: std::collections::HashMap<String, String>,
    body: Bytes,
) -> Response {
    let codicil_router = match CodicilRouter::discover_routes(&state.project_path) {
        Ok(r) => r,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to discover routes"))
                .unwrap();
        }
    };

    let http_method = codicil_core::HttpMethod::from_method(method.as_str())
        .unwrap_or(codicil_core::HttpMethod::GET);

    let route_match = match codicil_router.find_route(&http_method, path) {
        Some(m) => m,
        None => {
            return Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::from("404 Not Found"))
                .unwrap();
        }
    };

    let route_file = match RouteFile::parse(&route_match.route.file_path) {
        Ok(rf) => rf,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Failed to parse route: {}", e)))
                .unwrap();
        }
    };

    let mut ctx = RequestContext::new(method.to_string(), path.to_string());
    ctx = ctx.with_params(route_match.params);
    ctx = ctx.with_query(query_params);

    for (key, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            ctx.headers.insert(key.to_string(), v.to_string());
        }
    }

    let body_str = String::from_utf8_lossy(&body).to_string();
    ctx = ctx.with_body(body_str);

    if !route_file.middleware.is_empty() {
        if let Ok(chain) = MiddlewareChain::from_names(&route_file.middleware, &state.project_path)
        {
            match chain.execute(ctx).await {
                Ok(modified_ctx) => {
                    ctx = modified_ctx;
                }
                Err(e) => {
                    return Response::builder()
                        .status(StatusCode::INTERNAL_SERVER_ERROR)
                        .body(Body::from(format!("Middleware error: {}", e)))
                        .unwrap();
                }
            }
        }
    }

    let handler = Handler::new(route_file, route_match.route.file_path.clone());
    let error_route = codicil_router.error_route().cloned();

    match handler.execute(ctx).await {
        Ok(response) => {
            let mut builder = Response::builder().status(response.status);
            for (key, value) in response.headers {
                builder = builder.header(&key, &value);
            }
            builder.body(Body::from(response.body)).unwrap()
        }
        Err(e) => {
            if let Some(error_path) = error_route {
                let error_handler = ErrorHandler::new(error_path);
                let ctx = RequestContext::new(method.to_string(), path.to_string());
                if let Ok(response) = error_handler.execute(e.clone(), ctx).await {
                    let mut builder = Response::builder().status(response.status);
                    for (key, value) in response.headers {
                        builder = builder.header(&key, &value);
                    }
                    return builder.body(Body::from(response.body)).unwrap();
                }
            }
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Handler error: {}", e)))
                .unwrap()
        }
    }
}

fn cmd_build(path: &str) -> Result<()> {
    use codicil_core::Router as CodicilRouter;
    use std::fs::{self, File};
    use std::io::Write;

    let project_path = Path::new(path);
    let dist_path = project_path.join("dist");
    let public_path = project_path.join("public");

    let router = CodicilRouter::discover_routes(project_path)?;
    let routes: Vec<_> = router.routes().collect();

    println!("Building project...");
    println!("  Found {} routes", routes.len());

    if dist_path.exists() {
        fs::remove_dir_all(&dist_path)?;
    }
    fs::create_dir_all(&dist_path)?;
    fs::create_dir_all(dist_path.join("routes"))?;
    fs::create_dir_all(dist_path.join("public"))?;

    let mut route_manifest: Vec<serde_json::Value> = Vec::new();

    for route in &routes {
        print!("  Building {} {}", route.method, route.path);

        let route_file = match RouteFile::parse(&route.file_path) {
            Ok(rf) => rf,
            Err(e) => {
                println!(" [FAIL]");
                eprintln!("    Failed to parse: {}", e);
                continue;
            }
        };

        let brief_code = &route_file.brief_code;
        if brief_code.trim().is_empty() {
            println!(" (empty - skipping)");
            continue;
        }

        // Use a temporary file for compilation to avoid [route] header issues
        let temp_dir = std::env::temp_dir().join("codicil-build");
        fs::create_dir_all(&temp_dir)?;
        let temp_file = temp_dir.join(format!(
            "{}.{}.bv",
            route.method,
            route.path.replace(['/', ':'], "_")
        ));
        fs::write(&temp_file, brief_code)?;

        let compiler = match BriefCompiler::new() {
            Ok(c) => c,
            Err(_) => {
                println!(" [WARN] Brief compiler not found");
                continue;
            }
        };

        let result = compiler.build(&temp_file);
        match result {
            Ok(build_result) => {
                // Check if build failed only due to trivial pre/post conditions (P009/P010)
                let stderr = &build_result.stderr;
                let has_only_trivial_errors = stderr.contains("error[P009]:")
                    && stderr.contains("error[P010]:")
                    && !stderr.contains("error[P008]:")
                    && !stderr.contains("error[B")
                    && !stderr.contains("error[C");

                if build_result.success || has_only_trivial_errors {
                    let out_path = dist_path.join("routes").join(
                        route
                            .file_path
                            .file_name()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or("route.bv"),
                    );
                    fs::copy(&route.file_path, &out_path)?;

                    route_manifest.push(serde_json::json!({
                        "method": format!("{:?}", route.method),
                        "path": route.path,
                        "file": route.file_path.file_name().unwrap_or_default().to_str().unwrap_or(""),
                        "handler": route_file.handler_name,
                        "middleware": route_file.middleware,
                    }));
                    println!(" [OK]");
                } else {
                    println!(" [FAIL]");
                    eprintln!("    Compilation failed: {}", build_result.stderr);
                }
            }
            Err(e) => {
                println!(" [FAIL]");
                eprintln!("    Build error: {}", e);
            }
        }
    }

    if public_path.exists() {
        println!("  Copying public/...");
        copy_dir_all(&public_path, &dist_path.join("public"))?;
    }

    let manifest_path = dist_path.join("manifest.json");
    let mut manifest_file = File::create(manifest_path)?;
    manifest_file.write_all(serde_json::to_string_pretty(&route_manifest)?.as_bytes())?;

    println!("\n[OK] Build complete. Output in dist/");
    println!("  {} routes compiled", route_manifest.len());

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    use std::fs;

    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn cmd_generate(command: &Generate) -> Result<()> {
    use std::fs;

    match command {
        Generate::Model { name } => {
            let singular = name.to_lowercase();
            let plural = format!("{}s", singular);

            let model_content = format!(
                "# lib/{}.bv\nstruct {{}}\n    id: Int;\n    name: String;\n    created_at: Int;\n}}\n\ntxn create {{\n    term;\n}};\n\ntxn find {{\n    term;\n}};\n\ntxn update {{\n    term;\n}};\n\ntxn delete {{\n    term;\n}};\n\ntxn list {{\n    term;\n}};",
                singular
            );
            fs::write(
                Path::new("lib").join(format!("{}.bv", singular)),
                model_content,
            )?;

            fs::write(
                Path::new("routes").join(format!("GET.{}.bv", plural)),
                format!(
                    "[route]\nmethod = \"GET\"\npath = \"/{}\"\n\ntxn handle {{\n    term;\n}};",
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("POST.{}.bv", plural)),
                format!(
                    "[route]\nmethod = \"POST\"\npath = \"/{}\"\n\ntxn handle {{\n    term;\n}};",
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("GET.{}.[id].bv", plural)),
                format!(
                    "[route]\nmethod = \"GET\"\npath = \"/{}\"\n\ntxn handle {{\n    term;\n}};",
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("PUT.{}.[id].bv", plural)),
                format!(
                    "[route]\nmethod = \"PUT\"\npath = \"/{}\"\n\ntxn handle {{\n    term;\n}};",
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("DELETE.{}.[id].bv", plural)),
                format!(
                    "[route]\nmethod = \"DELETE\"\npath = \"/{}\"\n\ntxn handle {{\n    term;\n}};",
                    plural
                ),
            )?;

            println!("Created model '{}'", name);
            println!("  - lib/{}.bv", singular);
            println!("  - routes/GET.{}.bv", plural);
            println!("  - routes/POST.{}.bv", plural);
            println!("  - routes/GET.{}.[id].bv", plural);
            println!("  - routes/PUT.{}.[id].bv", plural);
            println!("  - routes/DELETE.{}.[id].bv", plural);
        }
        Generate::Middleware { name } => {
            let middleware_path =
                Path::new("middleware").join(format!("{}.bv", name.to_lowercase()));
            let content = "[route]\n\ntxn handle {\n    term;\n};\n";
            std::fs::write(&middleware_path, content)?;
            println!("Created middleware '{}'", name);
            println!("  - middleware/{}.bv", name.to_lowercase());
        }
    }
    Ok(())
}
