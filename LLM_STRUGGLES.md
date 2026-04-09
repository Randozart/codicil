# LLM Struggles and Reflections: Developing Brief & Codicil

This document captures the technical and cognitive hurdles encountered by the LLM (opencode) during the development of the Codicil landing page and the associated Brief compiler enhancements.

## 1. Tooling & Environment Friction

### JSON Parsing & Escaping
The most frequent point of failure was the JSON interface for tool calls.
- **Problem**: Multi-line strings, Rust format strings (containing `{}`), and shell heredocs frequently caused "JSON Parse error: Expected '}'" or similar.
- **Workaround**: Switched to single-line Python scripts or writing temporary `.py` files to perform complex string manipulations on the codebase. This bypassed the JSON escaping issues inherent in the `edit` and `bash` tools.

### Precision Edits
- **Problem**: The `edit` tool requires exact string matching. In a growing codebase with complex indentation and repetitive patterns (like match arms), finding a unique `oldString` was difficult.
- **Effect**: Led to "thrashing" where I would attempt an edit, fail, read the file again, and repeat.

## 2. Language Learning Curve (Brief)

### Synthetic Memory vs. Reality
- **Problem**: As an LLM, I have patterns for common languages (Rust, JS, HTML). Brief is unique. I initially hallucinated syntax from other languages.
- **Examples**:
    - Used `#` for comments (Brief uses `//`).
    - Used `{{ }}` for interpolation (Brief uses `{ }`).
    - Assumed `.bv` files returned HTML (they return component names).
- **Solution**: The user maintained a `CORRECTIONS.md` and `BRIEF_REFERENCE.md`, which were vital for "re-centering" my understanding of the language semantics.

## 3. Rust-to-JavaScript Code Generation

### The "Double-Escape" Problem
The compiler is written in Rust. It generates Rust code (WASM), which in turn generates or interacts with JavaScript.
- **Problem**: Generating a Rust `format!` string that produces a JavaScript string which itself contains escaped characters (like JSON quotes) required three or four levels of backslashes.
- **Example**: `format!("{{\\\"op\\\":\\\"each\\\"}}")` in Rust to produce `{"op":"each"}` in JS.
- **Failure**: I frequently missed the correct number of braces or backslashes, leading to generated code that failed to compile in the final WASM stage.

### JS-Sys & Reflect API
- **Problem**: I initially assumed Rust's `js_sys::Object` behaved like a JS object with a `.get()` method. It does not.
- **Discovery**: Interacting with dynamic JS objects from Rust requires the `js_sys::Reflect` API (`Reflect::get`, `Reflect::set`). Finding this required a deep dive into the generated errors.

## 4. The Feedback Loop

### High Latency
- **Process**: Modify Compiler -> Rebuild Compiler -> Run Compiler on `.rbv` -> `wasm-pack` builds generated code -> Test in browser.
- **Problem**: A single typo in a generated `output.push_str()` call wouldn't manifest until the very last step. This made debugging the "Property Access" and "Reactive Loop" features particularly slow.

## 5. Architectural Complexity

### Implicit State vs. Explicit Declarations
- **Struggle**: Balancing the user's desire for "Brief code should just run" (automatic initialization) with the language's philosophy of "all values must be on the table" (explicit initialization).
- **Resolution**: I had to modify the Parser and Desugarer simultaneously to ensure that `let count: Int = 0;` was enforced in structs while still allowing contract-based sugar in global scope.

---

**Final Note**: The successful implementation of recursive property access and reactive timing was only possible because the user provided immediate, concrete feedback on compiler output and was willing to "reset" when I hit a tool-induced deadlock.
