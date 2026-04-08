# Codicil Development Process

## Overview

Codicil is a contract-driven web framework built on the Brief programming language. This document tracks the development process, decisions, and implementation details.

## Architecture

```
Brief (Language) → Contract verification, state machines, Hoare logic
    ↓
Rendered Brief (.rbv) → HTML templating, b-text, b-trigger, reactive DOM
    ↓
Codicil (Framework) → File-based routing, middleware, server/client boundary, FFI
```

### Boundary Clarification

- **Brief (`.bv`)** = Pure logic. No pixels, hex codes, or mouse clicks.
- **Rendered Brief (`.rbv`)** = Brief + HTML. Handles DOM events and projections.
- **Codicil** = Framework. Routes HTTP requests, composes middleware, provides FFI bindings.

The handshake:
1. Codicil catches HTTP request → routes to `.bv` handler
2. Brief verifies contracts → mutates state → settles
3. Codicil observes state change → updates response

## Development Phases

### Day 1: Project Foundation (2026-04-08)

**Goal**: CLI scaffolding + basic routing

**Delivered**:
- Workspace structure: `codicil-cli` + `codicil-core`
- CLI commands: `init`, `dev`, `build`, `generate`
- Router module with route discovery and pattern matching
- Project scaffolding (`codi init`)
- Generate commands (`model`, `component`, `middleware`)

**Files Created**:
```
codicil/
├── Cargo.toml (workspace)
├── codicil-cli/
│   ├── Cargo.toml
│   └── src/main.rs
├── codicil-core/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       └── router.rs
```

**Key Decisions**:
1. **Separate repos**: Codicil is a separate project from brief-compiler, imported as subprocess
2. **Rust workspace**: Two crates - cli (binary) and core (library)
3. **File-based routing**: Routes discovered from filesystem, not registered

**Test Results**: 6/6 router tests passing

**Next**: Day 2 - Brief compiler integration

### Day 2: Brief Compiler Integration (2026-04-08)

**Goal**: Call Brief compiler, parse TOML headers, execute handlers, verify contracts

**Delivered**:
- Route file parser (`route_file.rs`) - parses TOML headers and Brief code
- Brief compiler integration (`compiler.rs`) - calls `brief check` subprocess
- Request context (`context.rs`) - HTTP request/response abstraction
- Handler execution (`handler.rs`) - executes Brief handlers
- Middleware chain (`middleware.rs`) - middleware composition
- CLI integration - routes now dispatch to Brief handlers
- Axum HTTP server with route discovery

**Files Created/Modified**:
```
codicil-core/src/
├── route_file.rs      # NEW: Parse TOML headers + Brief code
├── compiler.rs         # NEW: Brief compiler subprocess
├── context.rs         # NEW: Request/Response types
├── handler.rs         # NEW: Execute Brief handlers
├── middleware.rs      # NEW: Middleware chain
├── lib.rs             # UPDATED: Export new modules
└── router.rs          # UPDATED: Enhanced routing

codicil-cli/src/main.rs  # UPDATED: Full HTTP server
```

**Key Decisions**:
1. **TOML sections**: `[route]`, `[pre]`, `[post]` for metadata, Brief code for logic
2. **Brief as subprocess**: `brief check` for validation, `brief build` for compilation
3. **Context threading**: RequestContext passed through middleware chain to handler

**Test Results**: 10/10 tests passing

**Next**: Day 3 - Dev server with hot reload, scaffolding commands

### Day 3: Hot Reload & Scaffolding (2026-04-08)

**Goal**: File watching for hot reload, complete scaffolding commands

**Delivered**:
- File watcher module (`watcher.rs`) - using notify crate for file system events
- Hot reload in dev server - watches routes/, lib/, middleware/, components/
- Full CRUD scaffolding for models (6 routes generated)
- Improved component templates with rstruct and bindings
- Graceful shutdown with Ctrl+C

**Files Created/Modified**:
```
codicil-core/src/
├── watcher.rs         # NEW: File watching with notify
├── lib.rs            # UPDATED: Export watcher module

codicil-cli/src/main.rs  # UPDATED: 
    - Full CRUD scaffolding (6 routes per model)
    - Hot reload with Ctrl+C support
    - Better component templates
```

**Key Decisions**:
1. **Hot reload approach**: Watch files in background, print discovery on change
2. **Scaffolding generates**: GET list, POST create, GET item, PUT update, DELETE delete
3. **File watching**: Uses notify crate with 200ms poll interval

**Test Results**: 10/10 tests passing

**Next**: Day 4 - FFI integration, database support

### Day 4: FFI Foundation and Middleware (2026-04-08)

**Goal**: Establish FFI infrastructure for web operations, improve middleware

**Delivered**:
- FFI module in codicil-core for JSON operations
- codicil-ffi crate for web-specific bindings
- TOML binding specification for Codicil FFI
- JSON parsing/stringification helpers
- FFI stub implementations for HTTP and database

**Files Created**:
```
codicil-ffi/
├── Cargo.toml
├── bindings/
│   └── codicil.toml      # FFI bindings spec
└── src/
    └── lib.rs            # FFI implementations

codicil-core/src/
├── ffi.rs                # JSON helpers, FFI types
└── lib.rs               # Export FFI module
```

**Key Decisions**:
1. **FFI as separate crate**: codicil-ffi provides web bindings
2. **TOML bindings**: Follow Brief's FFI TOML format
3. **Middleware for I/O**: HTTP/DB happen in middleware, not Brief code
4. **JSON FFI**: parse_json, to_json available to Brief via FFI

**Test Results**: 12/12 tests passing

**Next**: Day 5 - Full request/response cycle

### Day 5: Full Request/Response Cycle (2026-04-08)

**Goal**: Connect Brief handlers to actual HTTP responses, implement body parsing, query params

**Delivered**:
- Body extraction from HTTP requests using Axum's `Bytes` body extractor
- Query param extraction using `Query<HashMap<String, String>>`
- Proper middleware chaining with error short-circuiting
- Handler updated to use `brief build` for execution (not just `brief check`)
- Response parsing from Brief build output (JSON with status, body, headers)
- All 12 tests passing

**Files Modified**:
```
codicil-cli/src/main.rs    # Added body/query extraction, fixed middleware chaining
codicil-cli/Cargo.toml    # Added bytes dependency
codicil-core/src/handler.rs  # Use brief build, parse JSON response output
codicil-core/src/context.rs  # Removed unused import
```

**Next**: Day 6 - Implement actual HTTP/database FFI

### Day 6: HTTP and Database FFI (2026-04-08)

**Goal**: Implement actual HTTP/database FFI using reqwest/sqlx

**Delivered**:
- `http_get`: Blocking HTTP GET using reqwest with timeout
- `http_post`: Blocking HTTP POST with JSON body using reqwest
- `db_query`: PostgreSQL query execution using sqlx with parameter binding
- Both sync and async versions of HTTP functions
- Proper error handling with String error conversion
- Type inference fixes for sqlx Row iteration

**Files Modified**:
```
codicil-ffi/Cargo.toml   # Added reqwest, sqlx, tokio dependencies
codicil-ffi/src/lib.rs   # Implemented http_get, http_post, db_query
```

**Key Decisions**:
1. **Sync over async for FFI**: Brief FFI is synchronous, so we use blocking wrappers with tokio runtime
2. **Postgres only**: sqlx configured for PostgreSQL (most common for web apps)
3. **Parameter binding**: Support 0-3+ parameters with type coercion

**Test Results**: 12/12 tests passing

**Next**: Day 7 - Error handling, [error].bv catch-all routes, structured error responses

### Day 7: Error Handling and Error Routes (2026-04-08)

**Goal**: Implement [error].bv catch-all routes, structured error responses

**Delivered**:
- `ApiError` struct with code, message, details fields and helper constructors
- `ApiError::to_response()` converts to JSON Response
- Error route discovery (`[error].bv`) in router
- `ErrorHandler` for executing error routes with error context
- Handler errors passed to error routes with `error.code`, `error.message`, `error.details`
- Default error fallback when no error route exists
- Fixed router bug: `file_path` now correctly set to actual file path

**Files Modified**:
```
codicil-core/src/context.rs   # Added ApiError struct
codicil-core/src/router.rs   # Added error_route discovery, fixed file_path bug
codicil-core/src/handler.rs   # Added ErrorHandler, HandlerError is now Clone
codicil-core/src/lib.rs       # Export ErrorHandler
codicil-cli/src/main.rs      # Wire error routes in handle_request
```

**Error Route Format**:
```brief
# routes/[error].bv
txn handle [true][response.status > 0] {
    term &response {
        status: error.details.status | 500,
        body: {
            code: error.code,
            message: error.message,
            details: error.details
        }
    };
};
```

**Test Results**: 12/12 tests passing

**Next**: Day 8 - Production build command (codi build), optimization, static file serving

### Day 8: Production Build and Static File Serving (2026-04-08)

**Goal**: Production build command, static file serving, optimization

**Delivered**:
- Improved `codi build` to actually compile routes and output to dist/
- Route manifest generated (routes compiled, middleware, handlers)
- Static file serving from public/ directory via fallback_service
- Copy public/ files to dist/ during build
- Clean dist/ directory before rebuild

**Files Modified**:
```
codicil-cli/Cargo.toml   # Added serde, serde_json dependencies
codicil-cli/src/main.rs  # Improved cmd_build, static file serving
```

**Build Output Structure**:
```
dist/
├── manifest.json       # Route manifest with method, path, file, handler, middleware
├── routes/           # Copied route files
│   ├── GET.index.bv
│   └── ...
└── public/           # Copied static files
    └── ...
```

**Test Results**: 12/12 tests passing

### Day 2: Brief Compiler Integration

**Goal**: Call Brief compiler, parse TOML headers, verify pre/post conditions

**In Progress**:
- [ ] Call `brief compile <file>` as subprocess
- [ ] Parse TOML headers [route], [pre], [post]
- [ ] Build request context from HTTP request
- [ ] Verify preconditions before handler execution
- [ ] Verify postconditions after handler execution
- [ ] Middleware chain with contract verification

---

## Technical Notes

### Route File Naming Convention

```
routes/GET.index.bv      → GET /
routes/GET.users.bv       → GET /users
routes/GET.users.[id].bv  → GET /users/:id
routes/POST.users.bv      → POST /users
```

Pattern: `{METHOD}.{path segments with [param]}.bv`

### Dependencies

```toml
clap = { version = "4", features = ["derive"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
notify = "6"
toml = "0.8"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
anyhow = "1"
thiserror = "1"
```

## Repository

- **Location**: `/home/randozart/Desktop/Projects/codicil`
- **Remote**: Private GitHub repo
