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
