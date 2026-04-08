# Getting Started with Codicil

This guide will walk you through creating your first Codicil application.

## Prerequisites

- Rust 1.70 or later
- Brief compiler installed (`brief --version` should work)
- Codicil CLI installed (`codi --version` should work)

## Step 1: Create a New Project

```bash
codi init myapp
cd myapp
```

This creates a new project with the following structure:

```
myapp/
├── routes/
│   └── GET.index.bv    # Default homepage
├── lib/
├── middleware/
├── components/
├── public/
│   └── favicon.svg    # Your app's favicon
├── migrations/
└── codicil.toml       # Project configuration
```

## Step 2: Start the Dev Server

```bash
codi dev
```

You'll see:

```
🔍 Discovering routes...
🔍 Discovered 1 routes:
  GET /

🚀 Dev server running at http://localhost:3000
📝 Watching for file changes (Ctrl+C to stop)...
```

Visit `http://localhost:3000` to see your app.

## Step 3: Modify the Homepage

Open `routes/GET.index.bv` and change it:

```brief
[route]
method = "GET"
path = "/"

[post]
response.status == 200

txn handle [true][post] {
    term &response {
        status: 200,
        body: "<html><head><link rel='icon' type='image/svg+xml' href='/favicon.svg'></head><body><h1>Hello, Codicil!</h1></body></html>"
    };
};
```

The dev server automatically reloads when you save.

## Step 4: Create a New Route

Create a file `routes/GET.hello.bv`:

```brief
[route]
method = "GET"
path = "/hello"

[post]
response.status == 200

txn handle [true][post] {
    term &response {
        status: 200,
        body: "Hello, World!"
    };
};
```

The dev server will discover it automatically. Visit `http://localhost:3000/hello`.

## Step 5: Add a Route with Parameters

Create `routes/GET.hello.[name].bv`:

```brief
[route]
method = "GET"
path = "/hello/:name"

[pre]
params.name.len() > 0

[post]
response.status == 200

txn handle [pre][post] {
    term &response {
        status: 200,
        body: { "message": "Hello, " + params.name + "!" }
    };
};
```

Visit `http://localhost:3000/hello/Alice` to see `{"message":"Hello, Alice!"}`.

## Step 6: Build for Production

```bash
codi build
```

This compiles your routes and outputs to `dist/`:

```
dist/
├── manifest.json       # Route manifest
├── routes/            # Compiled routes
└── public/            # Static files
```

## Next Steps

- Read the [Routing Guide](./routing.md) for advanced routing
- Read the [Contracts Guide](./contracts.md) for pre/postconditions
- Read the [Middleware Guide](./middleware.md) for middleware chains
- Read the [FFI Guide](./ffi.md) for database and HTTP access
