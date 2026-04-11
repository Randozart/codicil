# Brief Language Corrections

A guide to避免 common misconceptions when writing Brief code for Codicil.

---

## 1. `rstruct` Meaning

**误解**: `rstruct` 是 "reactive struct" - 任何带有 bindings 的 struct。

**正确**: `rstruct` 代表 "render struct" - 它结合了 HTML view 和 state。Bindings 也可以在普通的 `let` 变量上使用。

**证据**: `todo.rbv` 使用普通的 `let items: List<String>` 配合 bindings 如 `b-text="item"`。

---

## 2. 双大括号 `{{ }}`

**误解**: Brief 使用 `{{ }}` 作为 transaction bodies。

**正确**: Brief 使用单大括号 `{ }`。Codicil 代码模板中的双大括号 (`"txn handle {{ term; }}"`) 是 Rust 字符串格式化转义 - 它们在输出中产生单大括号。

---

## 3. `<script>` vs `<script type="brief">`

**误解**: `.rbv` 文件需要 `<script type="brief">`。

**正确**: 对于 `.rbv` 文件，普通的 `<script>` 就足够了。Brief 语言是隐式的。

---

## 4. `.bv` 路由不应该返回 HTML

**误解**: `index.bv` 应该返回完整的 HTML 字符串。

**正确**: 路由应该返回 **数据** (component 名称)，而不是 HTML。`.rbv` component 提供实际的 HTML/UI。

**示例**:
```brief
txn handle [true][post] {
    term "landing";
};
```

---

## 5. 注释: `//` 不是 `#`

**误解**: Brief 使用 `#` 作为注释。

**正确**: Brief 使用 `//` 作为注释。`#` 风格已从编译器中移除。

---

## 6. CSS Import 实现

**注意**: CSS import 需要引号包围的路径。

**正确语法**:
```brief
import "./file.css";
```

---

## 7. 简化的 Precondition

**建议**: `[true]` 是最简单的 precondition，表示 "始终为真"。

**正确语法**:
```brief
txn handle [true][post] {
    term "value";
};
```

---

## 8. SVG Import

**状态**: 部分实现在 AST 和 import resolver 中。

---

## Brief 关键语法总结

| 功能 | 正确语法 |
|---------|---------------|
| 注释 | `// comment` |
| Script tag | `<script>` (不是 `<script type="brief">`) |
| Transaction 大括号 | `{ }` (不是 `{{ }}`) |
| Return 值 | `term "value";` |
| CSS import | `import "./file.css";` (带引号的路径) |
| 路由 return | `term "component_name";` (数据，不是 HTML) |
| 参数类型 | `params.id is int` |
| Precondition | `[pre]` 或 `[true]` |
| Postcondition | `[post]` |

---

## 教训

1. 始终检查实际的编译器行为，而不是假设
2. spec/ 中的文档是权威的，但要用编译器源码验证
3. CSS/SVG imports 有文档记录但需要验证实现存在
4. 不要跳过 proof verification - 它能捕获真正的问题
5. Rust 字符串格式化使用 `{{ }}` 来转义单大括号 - 这不是 Brief 语法

---

## Files Modified During Implementation

1. **brief-compiler/src/ast.rs**: Added `Stylesheet` and `SvgComponent` variants
2. **brief-compiler/src/import_resolver.rs**: Handle CSS/SVG file imports
3. **brief-compiler/src/parser.rs**: Support quoted string import paths
4. **brief-compiler/src/typechecker.rs**: Handle new AST variants
5. **brief-compiler/src/annotator.rs**: Handle new AST variants
6. **brief-compiler/src/main.rs**: Extract CSS from imports, combine with inline styles