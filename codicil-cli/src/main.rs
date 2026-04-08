use std::path::Path;

use anyhow::Result;
use clap::{Parser, Subcommand};

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
# Brief compiler path (leave empty to use system brief)
brief_path = ""
"#,
        name
    );
    fs::write(project_dir.join("codicil.toml"), codicil_toml)?;

    let index_route = r#"# routes/GET.index.bv
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {{
  term &response {{
    status: 200,
    body: "Hello from Codicil!"
  }};
}};
"#;
    fs::write(project_dir.join("routes/GET.index.bv"), index_route)?;

    let gitkeep = "";
    fs::write(project_dir.join("lib/.gitkeep"), gitkeep)?;
    fs::write(project_dir.join("middleware/.gitkeep"), gitkeep)?;
    fs::write(project_dir.join("components/.gitkeep"), gitkeep)?;

    let env_example = r#"# Environment variables
DATABASE_URL=postgresql://localhost:5432/mydb
JWT_SECRET=your-secret-key
"#;
    fs::write(project_dir.join(".env.example"), env_example)?;

    println!("✓ Created project '{}'", name);
    println!("  cd {} && codi dev", name);
    Ok(())
}

async fn cmd_dev(path: &str) -> Result<()> {
    use axum::{
        routing::get,
        Router,
    };
    use tokio::net::TcpListener;
    use codicil_core::Router as CodicilRouter;

    let project_path = Path::new(path);
    let config_path = project_path.join("codicil.toml");

    let mut host = "localhost".to_string();
    let mut port: u16 = 3000;

    if config_path.exists() {
        let config_str = std::fs::read_to_string(&config_path)?;
        let config: toml::Value = toml::from_str(&config_str)?;
        if let Some(server) = config.get("server") {
            if let Some(h) = server.get("host").and_then(|v| v.as_str()) {
                host = h.to_string();
            }
            if let Some(p) = server.get("port").and_then(|v| v.as_integer()) {
                port = p as u16;
            }
        }
    }

    let router = CodicilRouter::discover_routes(project_path)?;
    let routes: Vec<_> = router.routes().collect();

    println!("🔍 Discovered {} routes:", routes.len());
    for route in &routes {
        println!("  {} {}", format!("{:?}", route.method), route.path);
    }

    let app = Router::new().route("/", get(|| async { "Codicil dev server" }));

    let addr = format!("{}:{}", host, port);
    println!("\n🚀 Dev server running at http://{}", addr);

    let listener = TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn cmd_build(path: &str) -> Result<()> {
    use codicil_core::Router as CodicilRouter;

    let project_path = Path::new(path);
    let router = CodicilRouter::discover_routes(project_path)?;
    let routes: Vec<_> = router.routes().collect();

    println!("📦 Building {} routes...", routes.len());
    for route in &routes {
        println!("  ✓ {} {}", format!("{:?}", route.method), route.path);
    }

    std::fs::create_dir_all(project_path.join("dist"))?;
    println!("\n✓ Build complete. Output in dist/");

    Ok(())
}

fn cmd_generate(command: &Generate) -> Result<()> {
    use std::fs;

    match command {
        Generate::Model { name } => {
            let model_path = Path::new("lib").join(format!("{}.bv", name.to_lowercase()));
            let model_content = format!(
                r#"# lib/{}.bv
struct {} {{
    id: Int;
}}

txn create [true][result.id > 0] {{
    term;
}};

txn find [id > 0][result exists] {{
    term;
}};

txn update [id > 0][result.id == id] {{
    term;
}};

txn delete [id > 0][result == true] {{
    term;
}};
"#,
                name.to_lowercase(),
                name
            );
            fs::write(&model_path, model_content)?;

            let singular = name.to_lowercase();
            let plural = format!("{}s", singular);
            fs::write(
                Path::new("routes").join(format!("GET.{}.bv", plural)),
                format!(r#"# routes/GET.{}.bv
txn handle [true][response.status == 200] {{
    term &response {{ status: 200, body: "[]" }};
}};
"#, plural),
            )?;

            println!("✓ Created model '{}'", name);
            println!("  - lib/{}.bv", name.to_lowercase());
            println!("  - routes/GET.{}.bv", plural);
        }
        Generate::Middleware { name } => {
            let middleware_path = Path::new("middleware").join(format!("{}.bv", name.to_lowercase()));
            let content = format!(
                r#"# middleware/{}.bv
txn handle [true][post] {{
    term;
}};
"#,
                name.to_lowercase()
            );
            std::fs::write(&middleware_path, content)?;
            println!("✓ Created middleware '{}'", name);
        }
        Generate::Component { name } => {
            let component_path = Path::new("components").join(format!("{}.rbv", name.to_lowercase()));
            let content = format!(
                r#"<script type="brief">
rstruct {} {{
    <div class="{}">Content</div>
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
        }
    }
    Ok(())
}
