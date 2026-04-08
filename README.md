# Codicil

**A contract-driven web framework built on Brief**

Codicil is a full-stack web framework that brings the power of contract-driven programming to web development. Built on the [Brief](https://github.com/anomalyco/brief-lang) programming language, Codicil enables developers to write web applications with verifiable preconditions, postconditions, and state transitions.

## Features

- **Contract-Driven Development**: Write routes with explicit preconditions and postconditions that are automatically verified
- **Hot Reload**: Instant feedback during development with file watching
- **FFI Integration**: Seamless access to HTTP, database, and external services through Brief's FFI system
- **Component System**: Build reusable UI components with Brief's reactive bindings
- **Error Handling**: Structured error responses with custom `[error].bv` catch-all routes

## Installation

### Prerequisites

- Rust 1.70+
- Brief compiler (`brief`)

### Quick Install

```bash
# Install Brief compiler
git clone https://github.com/anomalyco/brief-lang.git
cd brief-lang && cargo build --release
cp target/release/brief-compiler ~/.local/bin/brief

# Install Codicil CLI
git clone https://github.com/anomalyco/codicil.git
cd codicil && cargo build --release
cp target/release/codi ~/.local/bin/codi
```

## Quick Start

```bash
# Create a new project
codi init myproject
cd myproject

# Start development server
codi dev
```

Visit `http://localhost:3000` to see your application.

## Project Structure

```
myproject/
├── routes/           # Route files (*.bv)
├── lib/              # Shared library (*.bv)
├── middleware/       # Middleware (*.bv)
├── components/       # UI components (*.rbv)
├── migrations/       # Database migrations
├── public/          # Static files
└── codicil.toml     # Project configuration
```

## Writing Routes

Routes are defined in `.bv` files with TOML metadata and Brief code:

```brief
[route]
method = "GET"
path = "/users/:id"
middleware = ["auth", "ratelimit"]

[pre]
params.id is int

[post]
response.status == 200

txn handle [pre][post] {
    term &response {
        status: 200,
        body: { "user_id": params.id }
    };
};
```

### Route File Naming

| File | Method | Path |
|------|--------|------|
| `GET.index.bv` | GET | `/` |
| `GET.users.bv` | GET | `/users` |
| `GET.users.[id].bv` | GET | `/users/:id` |
| `POST.users.bv` | POST | `/users` |
| `[error].bv` | * | Error catch-all |

## Contracts in Routes

### Preconditions (`[pre]`)

Define what must be true before the handler executes:

```brief
[pre]
params.id > 0 && params.id < 1000000
```

### Postconditions (`[post]`)

Define what must be true after the handler completes:

```brief
[post]
response.status == 200
```

## Middleware

Create reusable middleware in `middleware/`:

```brief
# middleware/auth.bv
[route]

txn handle [true][post] {
    term;
};
```

Use middleware in routes:

```brief
[route]
middleware = ["auth", "cors"]
```

## Error Handling

Create a catch-all error handler:

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

## CLI Commands

```bash
codi init <name>           # Create new project
codi dev [path]           # Start dev server with hot reload
codi build [path]          # Production build
codi generate model <name>   # Scaffold model (6 routes)
codi generate middleware <name>  # Create middleware
codi generate component <name>   # Create RBV component
```

## Architecture

```
Brief (.bv) → Contract verification → State mutations
    ↓
Rendered Brief (.rbv) → HTML templating → DOM bindings
    ↓
Codicil → HTTP routing → Middleware → FFI → Response
```

## License

MIT
