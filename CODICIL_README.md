# Codicil Framework - Complete Specification

**Codicil** is a full-stack web framework built on Brief, a contract-driven language. This documentation provides everything needed to understand, use, and build Codicil.

---

## What is Codicil?

Codicil is a web framework that brings **contract-driven development** to full-stack web applications. Every route declares what it requires (preconditions) and guarantees (postconditions), and the compiler verifies these at compile time.

**Key Features:**
- 🛣️ **File-based routing** — Drop a `.bv` file in `routes/` and it becomes an endpoint
- 📝 **Contract-driven** — Pre/post conditions verified by compiler, not runtime
- 🔗 **Composable middleware** — Middleware contracts chain together; completeness verified
- 🔀 **Server/client boundary** — Context headers enforce separation; compiler prevents violations
- 🎨 **Type-safe responses** — Response shapes are typed; mismatches are compile errors
- ⚡ **Zero ceremony** — Filesystem is the API surface; no registration needed

---

## Documentation Structure

### For Framework Users

| Document | Audience | Purpose |
|----------|----------|---------|
| [CODICIL_FRAMEWORK_SPEC.md](./CODICIL_FRAMEWORK_SPEC.md) | Developers using Codicil | Complete specification of routing, contracts, middleware, server/client boundaries, etc. Start here. |
| [CODICIL_API_REFERENCE.md](./CODICIL_API_REFERENCE.md) | API users | Detailed API reference for all types, built-in functions, standard library, error handling |
| [CODICIL_EXAMPLES.md](./CODICIL_EXAMPLES.md) | Learners | Practical examples: basic GET/POST, CRUD, authentication, error handling, forms, components, advanced patterns |

### For Framework Builders

| Document | Audience | Purpose |
|----------|----------|---------|
| [CODICIL_IMPLEMENTATION_GUIDE.md](./CODICIL_IMPLEMENTATION_GUIDE.md) | Rust developers implementing Codicil | Complete technical specifications, build phases, core modules, integration points, 10-phase development roadmap |

---

## Quick Start

### Installation

```bash
$ codi init my-app
$ cd my-app
$ codi dev
```

Server running on `http://localhost:3000`

### Your First Route

```brief
# routes/GET.index.bv
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {
  term &response {
    status: 200,
    body: "Welcome to Codicil!"
  };
};
```

Test:
```bash
$ curl http://localhost:3000
Welcome to Codicil!
```

### Basic CRUD

```bash
$ codi generate:model Post
✓ Created lib/post.bv
✓ Created routes/GET.posts.bv
✓ Created routes/POST.posts.bv
✓ Created routes/GET.posts.[id].bv
✓ Created routes/PUT.posts.[id].bv
✓ Created routes/DELETE.posts.[id].bv
```

---

## Core Concepts

### 1. Contracts

Every route declares preconditions and postconditions:

```brief
[route]
[pre]
params.id is int

[post]
response.status == 200 || response.status == 404

txn handle [pre][post] {
  # Code must satisfy: IF preconditions THEN postconditions
  # Compiler verifies this
  term &response { ... };
};
```

**Why contracts?**
- ✅ Compile-time verification (no runtime surprises)
- ✅ Self-documenting code (pre/post explain intent)
- ✅ Impossible invalid states (compiler rejects bad paths)
- ✅ No defensive coding (preconditions guarantee input, postconditions guarantee output)

### 2. File-Based Routing

Routes are defined by their filesystem location:

```
routes/
  GET.index.bv                    → GET /
  GET.users.[id].bv               → GET /users/:id
  POST.users.bv                   → POST /users
  DELETE.users.[id].bv            → DELETE /users/:id
  GET.users.[id].posts.[post_id].bv → GET /users/:id/posts/:post_id
  [error].bv                      → * (catch-all)
```

**Benefits:**
- 🔍 Entire API visible at a glance
- ❌ No registration ceremony
- ✅ Cannot forget to wire up a route
- 📁 Filesystem structure mirrors API structure

### 3. Middleware Composition

Middleware contracts chain together; compiler verifies completeness:

```brief
# middleware/cors.bv
txn apply_cors [true]
               [response.headers.{"access-control-allow-origin"} exists] {
  # Adds CORS headers to response
  term;
};

# middleware/auth.bv
txn verify_jwt [context.headers.authorization exists]
               [result == authenticated(user) || result == unauthenticated] {
  # Adds user to context IF jwt valid
  term &result;
};

# routes/GET.protected.bv
[route]
middleware = ["cors", "auth"]
[pre]
context.user exists  # Guaranteed by auth middleware

txn handle [pre][response.status == 200] {
  # User is guaranteed to exist here
  term &response { ... };
};
```

**Compiler verifies:**
1. ✅ `cors` runs first, output satisfies `auth` preconditions
2. ✅ `auth` output satisfies handler preconditions
3. ❌ If chain incomplete → compilation error

### 4. Request Context

All handlers have implicit `context` available:

```brief
struct RequestContext {
  method: String,           # HTTP method
  path: String,             # Full path
  params: {[String]: String},  # Dynamic segments
  query: {[String]: String},   # Query params
  body: String,             # Raw body
  headers: {[String]: String}, # Headers
  user: Any?,               # From middleware (optional)
  session: Any?,            # From middleware (optional)
};
```

Access in handlers:

```brief
txn handle [true][response.status >= 200] {
  let id = context.params.{"id"};
  let page = context.query.{"page"};
  let user = context.user;  # From auth middleware
  let token = context.headers.{"authorization"};
  
  term &response { ... };
};
```

### 5. Server/Client Boundary

Context header determines execution location:

```brief
# Server context (default)
[route]
context = "server"      # Runs on server
# Has DB access, secrets, FFI bindings

# Client context
[component]
context = "client"      # Runs in browser
# No DB access, renders to JavaScript
```

**Compiler enforces:**
- ✅ Server can import/render client components
- ❌ Client cannot import server routes
- ❌ Client cannot access database

### 6. FFI for External Services

Database, auth, HTTP all go through type-safe FFI:

```toml
# codicil.toml
[ffi.database]
type = "rust"
library = "sqlx"
mapper = "database"
```

```brief
# lib/db.bv
frgn sig query_user(id: Int) -> String;
frgn sig insert_user(name: String, email: String) -> String;

txn find_user [id > 0]
              [result.id == id] {
  let json = query_user(id);
  let user = parse_user_from_json(json);
  term &user;
};
```

**Why FFI?**
- ✅ Brief stays pure (no side effects in core language)
- ✅ Type boundaries explicit
- ✅ Serialization explicit (no magic)
- ✅ Easy to test (mock FFI bindings)

---

## CLI Commands

### Project Management

```bash
codi init <name>              # Create new project
codi dev                       # Start dev server (hot reload)
codi build                     # Build for production
codi test                      # Run tests
```

### Scaffolding

```bash
codi generate:model <name>     # Create CRUD model + routes
codi generate:middleware <name> # Create middleware
codi generate:component <name> # Create component
```

### Database

```bash
codi migrate up                # Apply migrations
codi migrate down              # Revert migration
codi migrate status            # Show status
```

---

## Project Structure

```
my-app/
├── codicil.toml              # Project config
├── routes/                   # Public API routes
│   ├── GET.index.bv
│   ├── POST.users.bv
│   ├── GET.users.[id].bv
│   └── [error].bv
├── lib/                      # Internal modules (not routable)
│   ├── db.bv                 # Database layer
│   ├── errors.bv             # Error types
│   └── forms.bv              # Form validation
├── middleware/               # Middleware modules
│   ├── auth.bv
│   └── cors.bv
├── components/               # Reusable components
│   ├── layout.bv
│   └── user-card.bv
├── migrations/               # Database migrations
│   └── 001_create_users.sql
├── public/                   # Static assets
├── tests/                    # Test files
├── .env                      # Environment (not committed)
└── .env.production           # Production env
```

---

## Common Patterns

### Basic GET

```brief
# routes/GET.users.[id].bv
[route]
[pre]
params.id is int

[post]
response.status == 200 || response.status == 404

txn get_user [pre][post] {
  let user = db::find_user(parse_int(context.params.id));
  match user {
    found(u) => term &response { status: 200, body: to_json(u) };
    not_found => term &response { status: 404, body: "{}" };
  };
};
```

### Create with Validation

```brief
# routes/POST.users.bv
[route]
[pre]
context.body exists

[post]
response.status == 201 || response.status == 400

txn create_user [pre][post] {
  let form = parse_form(context.body);
  let validation = validate_user_form(form);
  
  match validation {
    valid => {
      let user = db::create_user(form.email, form.name);
      term &response { status: 201, body: to_json(user) };
    }
    invalid => {
      term &response { status: 400, body: to_json({error: "Invalid"}) };
    }
  };
};
```

### Protected Route with Auth

```brief
# routes/GET.profile.bv
[route]
middleware = ["auth"]

[pre]
context.user exists

[post]
response.status == 200

txn get_profile [pre][post] {
  let profile = db::get_user_profile(context.user.id);
  term &response { status: 200, body: to_json(profile) };
};
```

### Error Handling

```brief
# routes/[error].bv
[route]
[post]
response.status == 404

txn handle_not_found [true][post] {
  term &response {
    status: 404,
    body: to_json({error: "Not found", path: context.path})
  };
};
```

---

## Philosophy

Codicil follows three core principles:

### 1. Contracts Are Ground Truth

- ✅ Define contracts first (pre/post conditions)
- ✅ Code must satisfy contracts
- ❌ Never weaken contracts to match lazy code

```brief
# CORRECT: Specific contract with specific code
[pre]
params.id is int

txn handle [pre][response.status == 200] {
  let id = parse_int(context.params.id);
  term &response { ... };
};

# WRONG: Weakened contract to match incomplete code
[pre]
true  # Useless

txn handle [true][response.status == 200] {
  # No validation
  term &response { ... };
};
```

### 2. Zero Ceremony, Maximum Visibility

- ✅ Filesystem is configuration
- ✅ Every file in `routes/` is a public endpoint
- ✅ Entire API visible at a glance

```
routes/GET.index.bv
routes/GET.users.[id].bv
routes/POST.users.bv
routes/DELETE.users.[id].bv
```

No registry, no wiring, no forgotten routes.

### 3. Explicit Over Implicit

- ✅ Imports are explicit (no global state)
- ✅ Middleware dependencies declared
- ✅ Error types explicit (unions, not exceptions)
- ✅ Request context explicit (not magic global)

---

## Architecture

### Request Flow

```
HTTP Request
    ↓
Router (filesystem → endpoint)
    ↓
Middleware Chain (CORS, Auth, etc.)
    ↓
Request Context Builder (parse query, body, headers)
    ↓
Brief Compiler (verify preconditions)
    ↓
Handler Execution (route transaction)
    ↓
Brief Compiler (verify postconditions)
    ↓
Response Serialization (JSON, HTML)
    ↓
HTTP Response
```

### Compiler Integration

Brief compiler is used for:
1. **Type checking** — Request/response shapes must match types
2. **Contract verification** — Pre/post conditions verified on all paths
3. **Path analysis** — All execution paths checked
4. **Error reporting** — Compile-time errors caught early

---

## Key Differences from Other Frameworks

| Feature | Codicil | Express | Django | Next.js |
|---------|---------|---------|--------|---------|
| Routing | File-based | Manual registry | File-based | File-based |
| Type Safety | Contracts ✅ | JS only ❌ | Optional ⚠️ | TS optional ⚠️ |
| Middleware | Contract-verified ✅ | Runtime ❌ | Decorators ⚠️ | Vercel functions ⚠️ |
| Config | TOML ✅ | JavaScript ❌ | Python ❌ | JSON ⚠️ |
| Error Handling | Typed unions ✅ | Exceptions ❌ | Exceptions ❌ | Try/catch ⚠️ |
| Validation | Contracts ✅ | Manual ❌ | Validators ⚠️ | Manual ❌ |
| Server/Client | Enforced boundary ✅ | Manual separation ❌ | Server only ❌ | Next boundaries ⚠️ |

---

## Getting Help

### Documentation Files

1. **[CODICIL_FRAMEWORK_SPEC.md](./CODICIL_FRAMEWORK_SPEC.md)** — Full framework specification (start here)
2. **[CODICIL_API_REFERENCE.md](./CODICIL_API_REFERENCE.md)** — Complete API reference
3. **[CODICIL_EXAMPLES.md](./CODICIL_EXAMPLES.md)** — Practical examples and tutorials
4. **[CODICIL_IMPLEMENTATION_GUIDE.md](./CODICIL_IMPLEMENTATION_GUIDE.md)** — For framework builders

### Questions?

- 📖 Read the framework spec first
- 📚 Check examples for your use case
- 🔍 Browse API reference for function signatures
- 🛠️ If building the framework, see implementation guide

---

## Status

**Current Status**: Framework Specification Complete

Codicil specification is now ready for implementation. All core concepts, APIs, examples, and build phases are documented. An AI agent or development team can pick up the specification and implement the framework.

---

## Summary

Codicil brings **contract-driven development** to web applications:

- ✅ **Compile-time safety** — No runtime type errors or invalid states
- ✅ **File-based routing** — Filesystem is configuration
- ✅ **Composable middleware** — Verifiable contract chains
- ✅ **Type-safe request/response** — Mismatches are compile errors
- ✅ **Zero ceremony** — No registration, no magic
- ✅ **FFI for externals** — Type-safe database, auth, HTTP

**For Users**: Read [CODICIL_FRAMEWORK_SPEC.md](./CODICIL_FRAMEWORK_SPEC.md) to get started.

**For Builders**: Read [CODICIL_IMPLEMENTATION_GUIDE.md](./CODICIL_IMPLEMENTATION_GUIDE.md) to implement.

**For Learning**: Read [CODICIL_EXAMPLES.md](./CODICIL_EXAMPLES.md) for practical examples.

---

Happy building! 🚀
