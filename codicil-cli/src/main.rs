use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::{
    body::Body,
    extract::{Path as AxumPath, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{Response, IntoResponse},
    routing::any,
    Router,
};
use tower_http::services::ServeDir;
use bytes::Bytes;
use clap::{Parser, Subcommand};
use tokio::net::TcpListener;
use tokio::time::interval;
use codicil_core::{
    BriefCompiler, ErrorHandler, Handler, MiddlewareChain, RequestContext, 
    Router as CodicilRouter, RouteFile, is_relevant_file, watch_paths,
};

const FAVICON_SVG: &str = include_str!("../assets/favicon.svg");

#[derive(Parser)]
#[command(name = "codi")]
#[command(version = "0.1.0")]
#[command(about = "Codicil - A contract-driven web framework built on Brief")]
enum Cli {
    Init {
        name: String,
    },
    Dev {
        #[arg(default_value = ".")]
        path: String,
    },
    Build {
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
    Component { name: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli {
        Cli::Init { name } => {
            cmd_init(&name)?;
        }
        Cli::Dev { path } => {
            cmd_dev(&path).await?;
        }
        Cli::Build { path } => {
            cmd_build(&path)?;
        }
        Cli::Generate { command } => {
            cmd_generate(&command)?;
        }
    }

    Ok(())
}

fn cmd_init(name: &str) -> Result<()> {
    use std::fs;

    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    fs::create_dir_all(project_dir.join("routes"))?;
    fs::create_dir_all(project_dir.join("lib"))?;
    fs::create_dir_all(project_dir.join("middleware"))?;
    fs::create_dir_all(project_dir.join("components"))?;
    fs::create_dir_all(project_dir.join("migrations"))?;
    fs::create_dir_all(project_dir.join("public"))?;

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

    let index_route = r#"[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {{
  term &response {{
    status: 200,
    body: "<html><head><link rel='icon' type='image/svg+xml' href='/public/favicon.svg'></head><body>Hello from Codicil!</body></html>"
  }};
}};
"#;
    fs::write(project_dir.join("routes/GET.index.bv"), index_route)?;

    fs::write(project_dir.join("public/favicon.svg"), FAVICON_SVG)?;

    fs::write(project_dir.join("lib/.gitkeep"), "")?;
    fs::write(project_dir.join("middleware/.gitkeep"), "")?;
    fs::write(project_dir.join("components/.gitkeep"), "")?;

    let env_example = r#"# Environment variables
DATABASE_URL=postgresql://localhost:5432/mydb
JWT_SECRET=your-secret-key
"#;
    fs::write(project_dir.join(".env.example"), env_example)?;

    println!("✓ Created project '{}'", name);
    println!("  cd {} && codi dev", name);
    Ok(())
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

    let _ = BriefCompiler::new().ok();

    let state = AppState {
        project_path: Arc::new(project_path.clone()),
    };

    let addr = format!("{}:{}", host, port);
    
    println!("🔍 Discovering routes...");
    let codicil_router = CodicilRouter::discover_routes(&project_path)?;
    let routes: Vec<_> = codicil_router.routes().collect();
    println!("🔍 Discovered {} routes:", routes.len());
    for route in &routes {
        println!("  {} {}", format!("{:?}", route.method), route.path);
    }

    println!("\n🚀 Dev server running at http://{}", addr);
    println!("📝 Watching for file changes (Ctrl+C to stop)...\n");

    let watcher = watch_paths(&project_path).map_err(|e| anyhow::anyhow!("Failed to watch files: {}", e))?;

    let public_path = project_path.join("public");

    let app = Router::new()
        .route("/favicon.svg", any(handle_favicon))
        .route("/", any(handle_request))
        .route("/*path", any(handle_request))
        .with_state(state)
        .fallback_service(ServeDir::new(&public_path));

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
                println!("\n👋 Shutting down...");
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
                        println!("\n📄 File changed: {:?}", relevant);
                        println!("🔄 Route discovery triggered...");
                        match CodicilRouter::discover_routes(&project_path) {
                            Ok(router) => {
                                let routes: Vec<_> = router.routes().collect();
                                println!("✅ Discovered {} routes:", routes.len());
                                for route in &routes {
                                    println!("  {} {}", format!("{:?}", route.method), route.path);
                                }
                            }
                            Err(e) => {
                                println!("❌ Error discovering routes: {}", e);
                            }
                        }
                        println!("📝 Watching for file changes...");
                    }
                }
            }
        }
    }
    
    let _ = server_handle.abort();

    Ok(())
}

async fn handle_request(
    State(state): State<AppState>,
    method: Method,
    headers: HeaderMap,
    AxumPath(path): AxumPath<String>,
    Query(query_params): Query<std::collections::HashMap<String, String>>,
    body: Bytes,
) -> Response {
    let path = format!("/{}", path);
    
    let codicil_router = match CodicilRouter::discover_routes(&state.project_path) {
        Ok(r) => r,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from("Failed to discover routes"))
                .unwrap();
        }
    };

    let http_method = codicil_core::HttpMethod::from_str(method.as_str())
        .unwrap_or(codicil_core::HttpMethod::GET);
    
    let route_match = match codicil_router.find_route(&http_method, &path) {
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

    let mut ctx = RequestContext::new(method.to_string(), path.clone());
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
        if let Ok(chain) = MiddlewareChain::from_names(&route_file.middleware, &state.project_path) {
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
                let ctx = RequestContext::new(method.to_string(), path.clone());
                match error_handler.execute(e.clone(), ctx).await {
                    Ok(response) => {
                        let mut builder = Response::builder().status(response.status);
                        for (key, value) in response.headers {
                            builder = builder.header(&key, &value);
                        }
                        return builder.body(Body::from(response.body)).unwrap();
                    }
                    Err(_) => {}
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

    println!("📦 Building project...");
    println!("  Found {} routes", routes.len());

    if dist_path.exists() {
        fs::remove_dir_all(&dist_path)?;
    }
    fs::create_dir_all(&dist_path)?;
    fs::create_dir_all(dist_path.join("routes"))?;
    fs::create_dir_all(dist_path.join("public"))?;

    let mut route_manifest: Vec<serde_json::Value> = Vec::new();

    for route in &routes {
        print!("  Building {} {}", format!("{:?}", route.method), route.path);
        
        let route_file = match RouteFile::parse(&route.file_path) {
            Ok(rf) => rf,
            Err(e) => {
                println!(" ❌");
                eprintln!("    Failed to parse: {}", e);
                continue;
            }
        };

        let brief_code = &route_file.brief_code;
        if brief_code.trim().is_empty() {
            println!(" (empty - skipping)");
            continue;
        }

        let compiler = match BriefCompiler::new() {
            Ok(c) => c,
            Err(_) => {
                println!(" ⚠️  Brief compiler not found");
                continue;
            }
        };

        let result = compiler.build(&route.file_path);
        match result {
            Ok(build_result) => {
                if build_result.success {
                    let out_path = dist_path.join("routes").join(
                        route.file_path.file_name().unwrap_or_default().to_str().unwrap_or("route.bv")
                    );
                    fs::copy(&route.file_path, &out_path)?;
                    
                    route_manifest.push(serde_json::json!({
                        "method": format!("{:?}", route.method),
                        "path": route.path,
                        "file": route.file_path.file_name().unwrap_or_default().to_str().unwrap_or(""),
                        "handler": route_file.handler_name,
                        "middleware": route_file.middleware,
                    }));
                    println!(" ✓");
                } else {
                    println!(" ❌");
                    eprintln!("    Compilation failed: {}", build_result.stderr);
                }
            }
            Err(e) => {
                println!(" ❌");
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

    println!("\n✓ Build complete. Output in dist/");
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
                r#"# lib/{}.bv
struct {} {{
    id: Int;
    name: String;
    created_at: Int;
}}

txn create [name.len() > 0][result.id > 0] {{
    term;
}};

txn find [id > 0][result exists] {{
    term;
}};

txn update [id > 0 && name.len() > 0][result.id == id] {{
    term;
}};

txn delete [id > 0][result == true] {{
    term;
}};

txn list [true][result.len() >= 0] {{
    term;
}};
"#,
                singular, name
            );
            fs::write(Path::new("lib").join(format!("{}.bv", singular)), model_content)?;

            fs::write(
                Path::new("routes").join(format!("GET.{}.bv", plural)),
                format!(
                    r#"[route]
method = "GET"
path = "/{}"

txn handle [true][response.status == 200] {{
    term &response {{ status: 200, body: "[]" }};
}};
"#,
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("POST.{}.bv", plural)),
                format!(
                    r#"[route]
method = "POST"
path = "/{}"

txn handle [true][response.status == 201] {{
    term &response {{ status: 201, body: "{{\"id\": 1}}" }};
}};
"#,
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("GET.{}.[id].bv", plural)),
                format!(
                    r#"[route]
method = "GET"
path = "/{}"

[pre]
params.id is int

txn handle [pre][response.status == 200 || response.status == 404] {{
    term &response {{ status: 200, body: "{{\"id\": params.id}}" }} ;
}};
"#,
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("PUT.{}.[id].bv", plural)),
                format!(
                    r#"[route]
method = "PUT"
path = "/{}"

[pre]
params.id is int

txn handle [pre][response.status == 200] {{
    term &response {{ status: 200, body: "{{\"id\": params.id}}" }} ;
}};
"#,
                    plural
                ),
            )?;

            fs::write(
                Path::new("routes").join(format!("DELETE.{}.[id].bv", plural)),
                format!(
                    r#"[route]
method = "DELETE"
path = "/{}"

[pre]
params.id is int

txn handle [pre][response.status == 204] {{
    term &response {{ status: 204, body: "" }};
}};
"#,
                    plural
                ),
            )?;

            println!("✓ Created model '{}'", name);
            println!("  - lib/{}.bv", singular);
            println!("  - routes/GET.{}.bv", plural);
            println!("  - routes/POST.{}.bv", plural);
            println!("  - routes/GET.{}.[id].bv", plural);
            println!("  - routes/PUT.{}.[id].bv", plural);
            println!("  - routes/DELETE.{}.[id].bv", plural);
        }
        Generate::Middleware { name } => {
            let middleware_path = Path::new("middleware").join(format!("{}.bv", name.to_lowercase()));
            let content = format!(
                r#"[route]

txn handle [true][post] {{
    term;
}};
"#,
            );
            std::fs::write(&middleware_path, content)?;
            println!("✓ Created middleware '{}'", name);
            println!("  - middleware/{}.bv", name.to_lowercase());
        }
        Generate::Component { name } => {
            let component_path = Path::new("components").join(format!("{}.rbv", name.to_lowercase()));
            let content = format!(
                r#"<script type="brief">
rstruct {} {{
    value: String;

    txn update [true][post] {{
        term;
    }};

    <div class="{}">
        <span b-text="value">Empty</span>
        <button b-trigger:click="update">Update</button>
    </div>
}}
</script>

<view>
    <{} />
</view>
"#,
                name, name.to_lowercase(), name
            );
            std::fs::write(&component_path, content)?;
            println!("✓ Created component '{}'", name);
            println!("  - components/{}.rbv", name.to_lowercase());
        }
    }
    Ok(())
}
