# Codicil Framework - Implementation Guide

This guide provides detailed technical specifications for implementing the Codicil framework. It covers architecture, build phases, integration points, and key decisions.

**Audience**: Framework builders implementing Codicil from specification.

---

## Architecture Overview

### System Diagram

```
HTTP Request
    ↓
Router (filesystem → endpoint matching)
    ↓
Middleware Chain Executor (verify contracts)
    ↓
Request Context Builder (parse HTTP to struct)
    ↓
Brief Compiler (verify preconditions)
    ↓
Handler Transaction Execution
    ↓
Brief Compiler (verify postconditions)
    ↓
Response Serialization
    ↓
HTTP Response
```

### Core Modules

```
codicil/
├── cli/                          # Command-line interface
├── router/                       # Route discovery and dispatch
├── middleware/                   # Middleware chain execution
├── handler/                      # Route handler execution
├── compiler/                     # Brief compiler integration
├── ffi/                          # FFI bindings
├── dev/                          # Development server
├── config/                       # Configuration loading
└── project/                      # Project scaffolding
```

---

## Tech Stack Recommendations

### Language
- **Rust** — Same as Brief compiler, native performance, type safety

### Web Server
- **Axum** — Modern async runtime (tokio), easy composition
- Alternative: Actix-web (battle-tested, high performance)

### File Watching (dev server)
- **notify** or **watchexec** — For hot reload

### Database
- **sqlx** — Async database driver with tokio support

### Authentication
- **jsonwebtoken** — JWT encoding/decoding
- **argon2** or **bcrypt** — Password hashing

### Configuration
- **toml** crate — Parse codicil.toml files

### Logging
- **tracing** or **env_logger** — Structured logging

### Testing
- **tokio::test** — Async test framework
- **criterion** — Benchmarking

---

## 10-Phase Build Roadmap

### Phase 1: Project Structure & CLI Foundation (Week 1)

**Goal**: `codi init` scaffolds working project, `codi dev` runs HTTP server

**Tasks**:
- Create Rust project structure
- Implement `codi init <name>` command
  - Create directory structure (routes/, lib/, middleware/, components/, migrations/)
  - Write template codicil.toml
  - Create scaffold routes, middleware, components
  - Write .env file
- Implement basic HTTP server (Axum)
  - Listen on localhost:3000
  - Accept HTTP requests
  - Return basic 404 response
- Implement `codi dev` command
  - Start dev server
  - Print listening address
  - Basic error handling

**Testing**: `codi init my-app && cd my-app && codi dev` → Server listens on 3000

---

### Phase 2: File-Based Routing (Week 2)

**Goal**: Routes from filesystem are discovered and dispatched to handlers

**Tasks**:
- Implement path resolution
  - Parse `GET.users.[id].bv` → `GET /users/:id`
  - Handle nested directories
  - Dynamic segment extraction ([id] → context.params.id)
- Implement route loader
  - Scan routes/ directory recursively
  - Build routing table
  - Cache routes
- Implement request dispatcher
  - Match HTTP method + path to route file
  - Extract dynamic parameters
  - Pass to handler
- Implement basic handler executor (stub for now)

**Testing**: Create routes/GET.index.bv, routes/GET.users.[id].bv → Routes accessible via HTTP

---

### Phase 3: Brief Compiler Integration (Week 3)

**Goal**: Routes are type-checked and contract-verified by Brief compiler

**Tasks**:
- Integrate Brief compiler
  - Call Brief CLI or library to compile routes
  - Parse compiler output
  - Handle compilation errors
- Implement TOML header parsing
  - Extract [route], [pre], [post] sections
  - Parse method, path, middleware, context fields
- Implement request context building
  - Parse HTTP method, path, query, body, headers
  - Build RequestContext struct
  - Pass to handler
- Implement response validation
  - Type check response struct
  - Verify postcondition satisfaction
  - Convert to HTTP response

**Testing**: Routes with contracts compile, preconditions validated

---

### Phase 4: Middleware System (Week 4)

**Goal**: Middleware chain executes and contracts are verified

**Tasks**:
- Implement middleware loader
  - Scan middleware/ directory
  - Parse middleware declarations in codicil.toml
  - Load in order
- Implement middleware executor
  - Execute middleware transactions in sequence
  - Thread context through chain
  - Catch errors
- Implement contract chain verification
  - Middleware B's precondition ← Middleware A's postcondition
  - Route handler's precondition ← final middleware's postcondition
  - Compiler verifies entire chain
- Implement built-in middleware stubs
  - CORS middleware
  - Rate limit middleware
  - Session middleware
  - Auth middleware

**Testing**: Middleware runs in order, context threads through chain

---

### Phase 5: Hot Reload (Week 5)

**Goal**: Dev server detects file changes and recompiles without restart

**Tasks**:
- Implement file watcher
  - Watch routes/, lib/, middleware/, components/ directories
  - Debounce rapid changes (e.g., 200ms)
- Implement recompilation
  - Recompile changed files
  - Display compilation errors
  - Keep running if errors
- Implement client hot reload
  - Inject WebSocket client into HTML responses
  - Server notifies on changes
  - Browser reloads affected routes/components
- Optional: State preservation
  - Preserve client-side state across reload

**Testing**: Edit routes/GET.index.bv → Browser reloads automatically

---

### Phase 6: Server/Client Boundary (Week 6)

**Goal**: Routes declared `context = "server"` or `context = "client"` are handled correctly

**Tasks**:
- Implement context header parsing
  - Routes default to context = "server"
  - Components default to context = "client"
- Implement server-side route handling
  - Database access allowed
  - FFI bindings available
  - Render components to HTML strings
- Implement client-side component handling
  - Compile to JavaScript
  - No database access
  - Reactive signal support
- Implement boundary enforcement
  - Server cannot import client transactions
  - Client cannot import server routes
  - Compiler rejects violations

**Testing**: Server renders HTML, client renders JavaScript

---

### Phase 7: Database & FFI Integration (Week 7)

**Goal**: FFI bindings for database, auth, serialization work end-to-end

**Tasks**:
- Create FFI mappers
  - database mapper → SQLx functions
  - auth mapper → JWT/password functions
  - serialization mapper → JSON parsing
  - http mapper → HTTP client (for client-side)
- Implement database migration runner
  - `codi migrate up/down/status`
  - Track applied migrations (migrations_applied table)
- Implement FFI call handling
  - Routes call FFI signatures
  - Route to Rust implementation
  - Return results to Brief code
- Create example lib/db.bv
  - Sample database wrapper transactions

**Testing**: Create migration → `codi migrate up` → POST/GET routes use database

---

### Phase 8: Error Handling & Global Routes (Week 8)

**Goal**: Error types work, error boundaries catch unhandled errors

**Tasks**:
- Implement error page rendering
  - Compile errors displayed in browser
  - Runtime errors with stack trace (dev only)
- Implement [error].bv catch-all route
  - Handles unmatched paths (→ 404)
  - Handles unhandled errors
- Implement error response transformation
  - Union types → HTTP responses
  - Structured error JSON
- Implement logging
  - Request/response logging
  - Error logging
  - Dev console in browser

**Testing**: 404 errors caught by [error].bv → Error JSON returned

---

### Phase 9: CLI Scaffolding (Week 9)

**Goal**: `codi generate:*` commands scaffold boilerplate

**Tasks**:
- Implement `codi generate:model <name>`
  - Create lib/<name>.bv with struct + CRUD transactions
  - Create routes/GET.<name>s.bv (list)
  - Create routes/POST.<name>s.bv (create)
  - Create routes/GET.<name>s.[id].bv (read)
  - Create routes/PUT.<name>s.[id].bv (update)
  - Create routes/DELETE.<name>s.[id].bv (delete)
- Implement `codi generate:middleware <name>`
  - Create middleware/<name>.bv with template
  - Update codicil.toml
- Implement `codi generate:component <name>`
  - Create components/<name>.bv with template

**Testing**: `codi generate:model User` → 6 routes created with correct templates

---

### Phase 10: Production Build & Deployment (Week 10)

**Goal**: `codi build` creates production-ready output

**Tasks**:
- Implement `codi build`
  - Compile all routes with optimization
  - Bundle client components to JavaScript
  - Generate source maps (optional)
  - Output to dist/ directory
- Implement production server startup
  - `codi start` runs built app
  - No hot reload
  - Production logging
- Create deployment guide
  - Docker setup
  - Kubernetes setup
  - Vercel/serverless setup
- Add environment configuration
  - .env.production
  - Environment variable validation

**Testing**: `codi build && codi start` → Server runs, serves all routes

---

## Core Module APIs

### Router Module

```rust
pub struct Router {
    routes: HashMap<(HttpMethod, PathPattern), RoutePath>,
}

impl Router {
    pub fn discover_routes(project_root: &Path) -> Result<Self, RouterError> { ... }
    pub fn find_route(&self, method: &HttpMethod, path: &str) -> Option<RouteMatch> { ... }
    pub fn extract_params(&self, path: &str, pattern: &PathPattern) -> HashMap<String, String> { ... }
}

pub struct RouteMatch {
    pub route_file: PathBuf,
    pub params: HashMap<String, String>,
    pub handler_name: String,
}
```

### Middleware Module

```rust
pub struct MiddlewareChain {
    middleware: Vec<MiddlewareModule>,
}

impl MiddlewareChain {
    pub fn from_config(config: &CodicilConfig) -> Result<Self, MiddlewareError> { ... }
    pub async fn execute(&self, context: &mut RequestContext) -> Result<(), MiddlewareError> { ... }
    pub fn verify_contracts(&self, handler_preconditions: &Contract) -> Result<(), ContractError> { ... }
}
```

### Handler Module

```rust
pub struct Handler {
    pub route_path: PathBuf,
    pub method: HttpMethod,
    pub contract: Contract,
}

impl Handler {
    pub fn load(route_file: &Path) -> Result<Self, HandlerError> { ... }
    pub async fn execute(
        &self,
        http_request: &HttpRequest,
        context: RequestContext,
    ) -> Result<HttpResponse, HandlerError> { ... }
}

pub struct RequestContext {
    pub method: String,
    pub path: String,
    pub params: HashMap<String, String>,
    pub query: HashMap<String, String>,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub user: Option<Value>,
    pub session: Option<Value>,
}
```

### Compiler Module

```rust
pub struct BriefCompiler {
    compiler_path: PathBuf,
}

impl BriefCompiler {
    pub fn new() -> Result<Self, CompilerError> { ... }
    pub fn compile_route(&self, route_file: &Path) -> Result<CompiledRoute, CompilerError> { ... }
    pub fn verify_preconditions(&self, preconditions: &Contract, context: &RequestContext) -> Result<(), VerifyError> { ... }
    pub fn verify_postconditions(&self, postconditions: &Contract, response: &Response) -> Result<(), VerifyError> { ... }
}
```

### FFI Module

```rust
pub struct FfiBinder;

impl FfiBinder {
    pub async fn query_user(id: i32) -> Result<String, Error> { ... }
    pub async fn insert_user(name: &str, email: &str) -> Result<String, Error> { ... }
    pub fn jwt_encode(payload: &Value, secret: &str) -> Result<String, Error> { ... }
    pub fn to_json(obj: &Value) -> Result<String, Error> { ... }
    pub async fn http_get(url: &str) -> Result<String, Error> { ... }
}
```

---

## Integration with Brief

### Compilation Pipeline

1. Developer writes route: `routes/GET.users.[id].bv`
2. Codicil discovers route
3. Codicil calls Brief compiler: `brief compile routes/GET.users.[id].bv`
4. Brief returns: JavaScript, type defs, contract info
5. Codicil loads runtime and runs handler with request context
6. Brief verifies contracts at execution time
7. Response returned to client

### Key Integration Points

- **proof_engine.rs** → Contract verification logic
- **ffi/** → FFI binding and mapper system
- **ast.rs** → Brief type definitions
- **import_resolver.rs** → Module loading
- **wasm_gen.rs** → WASM compilation output

---

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_route_discovery() {
    let routes = Router::discover_routes(Path::new("test_project")).unwrap();
    assert_eq!(routes.len(), 5);
}

#[test]
fn test_dynamic_segment_extraction() {
    let params = Router::extract_params("/users/123/posts/456", PATTERN);
    assert_eq!(params["id"], "123");
}

#[test]
fn test_middleware_chain_contract_verification() {
    let chain = MiddlewareChain::from_config(&config).unwrap();
    let handler_pre = Contract { /* context.user exists */ };
    assert!(chain.verify_contracts(&handler_pre).is_ok());
}
```

### Integration Tests

```rust
#[tokio::test]
async fn test_full_request_flow() {
    let app = start_test_server().await;
    let response = app.get("/users/123").send().await.unwrap();
    assert_eq!(response.status(), 200);
}

#[tokio::test]
async fn test_middleware_chain_execution() {
    let app = start_test_server().await;
    let response = app
        .get("/protected")
        .header("Authorization", "Bearer invalid")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), 401);
}
```

### End-to-End Tests

```bash
$ codi init test-app
$ cd test-app
$ codi test
# All routes work correctly
# Middleware chain executes
# Contracts verified
# Errors handled
```

---

## Deployment

### Production Build

```bash
$ codi build --release
# Output: dist/
#   ├── app.js (bundled, minified)
#   ├── app.wasm (if using WASM backend)
#   ├── index.html
#   └── manifest.json
```

### Docker

```dockerfile
FROM rust:latest
WORKDIR /app
COPY . .
RUN cargo build --release
EXPOSE 3000
CMD ["./target/release/codicil", "start"]
```

### Environment Variables

```bash
DATABASE_URL=postgresql://prod-db.example.com/mydb
JWT_SECRET=<production-secret>
SESSION_SECRET=<production-secret>
LOG_LEVEL=info
```

---

## Summary for Builders

**10 Development Weeks**:
- Week 1-3: Routing, CLI, compiler integration
- Week 4-6: Middleware, hot reload, server/client boundary
- Week 7-9: Database/FFI, error handling, CLI scaffolding
- Week 10: Production build, deployment

**Core Modules**: Router, Middleware, Handler, Compiler, FFI, CLI, Dev, Config, Project

**Critical Success Factors**:
1. Tight Brief compiler integration for contract verification
2. Correct middleware chain contract threading
3. Type-safe request/response handling
4. Hot reload for excellent DX
5. FFI bindings for external services

**Reference Brief Resources**:
- proof_engine.rs — Contract verification
- ffi/ — FFI binding patterns
- import_resolver.rs — Module loading
- wasm_gen.rs — WASM compilation

Good luck building Codicil! 🚀
