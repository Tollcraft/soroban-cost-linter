# Writing a Custom Lint for `soroban-cost-linter`

> **What this guide covers**
> - When to add a new lint
> - Verifying scope and cost relevance
> - Registering the lint in the metadata registry
> - Implementing the lint using Dylint
> - Writing UI tests
> - Updating documentation and catalogs
>
---

## 1️⃣  Scope Verification

Before you start, ensure the pattern you want to detect is **not** already covered by:

- A **Clippy** lint (check `rustc --pretty=expanded` output or Clippy docs)
- An existing **soroban-cost-linter** lint (see `docs/lint_catalog.md`)
- A pattern that is **input‑dependent** (cost‑lints must be *input‑independent* to be safe for budgeting)

If the pattern fails any of the above checks, discuss with the maintainers first.

## 2️⃣  Environment Setup

```bash
# Install the Dylint toolchain (once per machine)
cargo install cargo-dylint dylint-link --version "^6.0.1"

# Clone the repository and enter it
git clone --filter=blob:none https://github.com/Tollcraft/soroban-cost-linter.git
cd soroban-cost-linter

# Ensure you are on the develop branch (or the branch you will PR against)
git checkout main
```

## 3️⃣  Register the Lint

All lints are listed in `soroban_cost_lints/src/lib.rs` inside the `LINT_METADATA` static map.

1. Open the file and add a new entry following the pattern:

```rust
("my_new_lint", LintMetadata {
    name: "my_new_lint",
    category: Category::Compute, // choose StorageOperations, Compute, Memory, or EntryLifecycle
    default_level: LintLevel::Warn,
    description: "Detects ...",
})
```

2. Choose a **snake_case** name that describes the pattern (e.g., `storage_in_loop`). Avoid the `soroban_` prefix unless required for uniqueness.

## 4️⃣  Implement the Lint

Create a new module under `soroban_cost_lints/src/lints/`:

```bash
mkdir -p soroban_cost_lints/src/lints/my_new_lint
touch soroban_cost_lints/src/lints/my_new_lint/mod.rs
```

Write the lint using the Dylint API. A minimal skeleton:

```rust
use dylint_linting::{LateContext, LateLintPass, LintContext};
use rustc_hir::{Expr, ExprKind};
use rustc_span::symbol::Symbol;

declare_lint! {
    /// Detects ...
    pub MY_NEW_LINT,
    Warn,
    "description of the lint"
}

#[derive(Default)]
pub struct MyNewLint;

impl<'tcx> LateLintPass<'tcx> for MyNewLint {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Example: look for `env.clone()` calls inside loops
        if let ExprKind::MethodCall(path, _, args, _) = expr.kind {
            if path.ident.name == Symbol::intern("clone") {
                // Additional checks: ensure we are inside a loop
                if cx.tcx.hir().parent_iter(expr.hir_id).any(|(parent, _)| matches!(parent.kind, rustc_hir::Node::Stmt(_))) {
                    cx.lint(MY_NEW_LINT, "`env.clone()` inside a loop is costly");
                }
            }
        }
    }
}

impl_lint_pass!(MyNewLint => [MY_NEW_LINT]);
```

Adapt the logic to your specific pattern – most existing lints follow a similar structure.

## 5️⃣  Add UI Tests

All lints have UI tests under `tests/ui/`. Create a new test file, e.g., `tests/ui/my_new_lint.rs`:

```rust
// should lint
fn bad() {
    let mut env = Env::default();
    for _ in 0..10 {
        env.clone(); // <- triggers `my_new_lint`
    }
}

// should not lint
fn good() {
    let env = Env::default();
    env.clone(); // fine outside loops
}
```

Run the UI test suite to ensure the lint fires correctly:

```bash
cargo test --test ui_my_new_lint
```

## 6️⃣  Update Documentation

1. **Add an entry to the lint catalog** (`docs/lint_catalog.md`):

```markdown
| `my_new_lint` | warn | Detects ... | [Link](lints/my_new_lint.md) |
```

2. **Create a lint reference page** (`docs/lints/my_new_lint.md`):

```markdown
# `my_new_lint`

- **Category**: Compute
- **Default severity**: `warn`
- **Description**: Detects ...
- **Example**:

```rust
// code that triggers the lint
```

- **Rationale**: Explain why this pattern is costly for Soroban contracts.
```

3. **Add a brief description to the `README.md`** under the "Built‑in Lints" table if one exists.

## 7️⃣  PR Checklist

- [ ] Lint is registered in `LINT_METADATA`.
- [ ] Implementation compiles on the pinned nightly toolchain.
- [ ] UI test(s) added and passing.
- [ ] Documentation added to the catalog and a dedicated lint page.
- [ ] `cargo fmt`, `cargo clippy -D warnings`, and `cargo test` all pass.
- [ ] PR description references this issue and follows the repository's PR template.

---

### 🎉  You’re Done!

Once the PR is merged, the new lint becomes part of the public API. Remember that lint names are stable – **do not rename** after release.

For any questions, feel free to open an issue or ping the maintainers.
