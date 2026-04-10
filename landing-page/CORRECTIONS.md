# Corrections Log

Documenting all corrections made during landing-page implementation.

## Session Date: 2026-04-09

### 1. rstruct Meaning

**Misconception**: `rstruct` meant "reactive struct" - any struct with bindings.

**Correction**: `rstruct` stands for "render struct" - it combines HTML view with state. Bindings work on regular `let` variables at script scope too.

**Evidence**: `todo.rbv` uses plain `let items: List<String>` with bindings like `b-text="item"`.

---

### 2. Double Braces `{{ }}`

**Misconception**: Brief uses `{{ }}` for transaction bodies.

**Correction**: Brief uses single braces `{ }`. The double braces in Codicil's code templates (`"txn handle {{ term; }}"`) are Rust string formatting escaping - they produce single braces in output.

---

### 3. `<script>` vs `<script type="brief">`

**Misconception**: `.rbv` files require `<script type="brief">`.

**Correction**: For `.rbv` files, plain `<script>` is sufficient. The Brief language is implicit.

---

### 4. `.bv` Routes Should Not Return HTML

**Misconception**: `index.bv` should return the full HTML string.

**Correction**: Routes should return **data** (component name), not HTML. The `.rbv` component provides the actual HTML/UI.

**Example**:
```brief
txn handle [true][post] {
    term "landing";
};
```

---

### 5. Comments: `//` not `#`

**Misconception**: Brief uses `#` for comments.

**Correction**: Brief uses `//` for comments. The `#` style was removed from the compiler.

---

### 6. CSS Import Was Not Implemented

**Issue**: `import "./landing.css"` failed because the import resolver only looked for `.bv` files.

**Fix**: 
- Added `Stylesheet(String)` and `SvgComponent(String)` to AST `TopLevel` enum
- Modified `import_resolver.rs` to detect `.css`/`.svg` extensions and read files directly
- Modified `parser.rs` to support quoted string paths like `"./file.css"`

---

### 7. Trivial Precondition Warning Logic

**Issue**: Compiler warned about `[true]` preconditions even when postcondition was non-trivial.

**Fix**: User fixed the compiler to only warn when BOTH precondition AND postcondition are trivial.

---

### 8. SVG Import Implementation

**Status**: Partially implemented in AST and import resolver.

**Remaining**: View compiler needs to generate wrapper component for SVG with prop passthrough.

---

### Summary of Key Syntax

| Feature | Correct Syntax |
|---------|---------------|
| Comments | `// comment` |
| Script tag | `<script>` (not `<script type="brief">`) |
| Transaction braces | `{ }` (not `{{ }}`) |
| Return value | `term "value";` |
| CSS import | `import "./file.css";` (quoted path) |
| Route return | `term "component_name";` (data, not HTML) |

---

### Files Modified

1. **brief-compiler/src/ast.rs**: Added `Stylesheet` and `SvgComponent` variants
2. **brief-compiler/src/import_resolver.rs**: Handle CSS/SVG file imports
3. **brief-compiler/src/parser.rs**: Support quoted string import paths
4. **brief-compiler/src/typechecker.rs**: Handle new AST variants
5. **brief-compiler/src/annotator.rs**: Handle new AST variants
6. **brief-compiler/src/main.rs**: Extract CSS from imports, combine with inline styles

---

### Lessons Learned

1. Always check the actual compiler behavior, not assumptions
2. The docs in `spec/` are authoritative, but verify with compiler source
3. CSS/SVG imports were documented but not implemented - verify implementation exists
4. Don't skip proof verification - it catches real issues
5. Rust string formatting uses `{{ }}` to escape single braces - this is not Brief syntax
