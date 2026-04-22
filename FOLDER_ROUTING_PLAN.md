# Codicil Implementation Plan

## Overview

This document tracks the implementation of folder-based routing for Codicil.

---

## Completed Implementation ✓

### 1. Folder-Based Routing (DONE)
- `src/page.rbv` → GET `/`
- `src/route.rbv` → All HTTP methods
- `[segment]/` → Dynamic routes
- `(group)/` → Route groups (excluded from URL)
- `_private/` → Private folders (ignored)
- Legacy `routes/` fallback supported

### 2. LSP Codicil Detection (DONE)
- Detects `.codicil/` folder alongside `codicil.toml`
- Loads `.codicil/config.toml` settings
- Codicil-specific autocomplete suggestions

### 3. b-show String Comparison Fix (DONE)
- Fixed in `brief-compiler` to support String, Bool, and numeric comparisons
- Type-aware expression evaluator in WASM runtime
- Supports: `lens == 'systems'`, `active == true`, `score > 10`

---

## Future Directives

### Planned
1. **`b-if`** - Conditional DOM insertion (not just visibility)
2. **`b-model`** - Two-way form binding
3. **`b-effect`** - Reactive side effects
4. **`b-on`** - General event binding
5. **`b-ref`** - Element references

### Expression Evaluator Enhancement
The fixed `eval_expr` function now supports:
- String comparisons (quoted literals)
- Boolean comparisons (`true`/`false`)
- Numeric comparisons (Int/Float)

This foundation enables future directives with complex expressions.

---

## File Changes Summary

| File | Change |
|------|--------|
| `codicil-core/src/router.rs` | Folder-based route discovery |
| `codicil-core/src/handler.rs` | RBV compilation support |
| `codicil-cli/src/main.rs` | `codi init` with `src/` structure |
| `brief-compiler/src/backend/wasm.rs` | Type-aware `eval_show` |

Folders prefixed with underscore `_` are excluded from routing:

```
src/_internal/helpers/page.rbv  →  (not a route)
```

---

## Part 2: LSP Codicil Detection Enhancement

### Current Detection

The LSP currently detects Codicil mode by looking for `codicil.toml` in parent directories.

### Enhanced Detection

Also check for `.codicil` folder at the project root:

```
.codicil/
├── config.toml       # Project configuration
└── hooks/            # Custom hooks (future)
```

### Implementation

1. Update `brief-compiler/src/lsp.rs`:
   - Check for `.codicil` folder alongside existing `codicil.toml` check
   - Load Codicil-specific settings from `.codicil/config.toml` if present

2. Codicil-specific LSP features:
   - Auto-complete Codicil route file headers
   - Validate `page.rbv` vs `route.rbv` conventions
   - Suggest correct file naming based on context

---

## Part 3: Migration & Backward Compatibility

### Support Both Directories

- **New**: `src/` (folder-based routing)
- **Legacy**: `routes/` (method.file.bv files)

Priority: `src/` takes precedence if both exist.

### Deprecation Path

1. Add warning when legacy `routes/` is used
2. Recommend migrating to `src/` structure
3. Future: Remove `routes/` support

---

## Implementation Order

1. **Update router discovery** (`codicil-core/src/router.rs`)
   - Scan `src/` directory
   - Parse `page.rbv` and `route.rbv` conventions
   - Handle route groups `()`
   - Handle private folders `_`

2. **Update CLI** (`codicil-cli/src/main.rs`)
   - Change default from `routes/` to `src/`
   - Update template generation
   - Support both directories during discovery

3. **Update LSP** (`brief-compiler/src/lsp.rs`)
   - Add `.codicil` folder detection
   - Load Codicil settings from `.codicil/config.toml`
   - Add Codicil-specific auto-complete suggestions

4. **Test & Document**
   - Create test cases for new routing
   - Update CLI documentation

---

## File Changes Summary

| File | Change |
|------|--------|
| `codicil-core/src/router.rs` | Rewrite route discovery for folder-based |
| `codicil-cli/src/main.rs` | Update CLI to use `src/` as default |
| `brief-compiler/src/lsp.rs` | Add `.codicil` folder detection |
| `brief-compiler/src/main.rs` | Ensure compiler respects Codicil settings |