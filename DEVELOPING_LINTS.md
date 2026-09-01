# Developing Lints

> This is the **single authoritative guide** for adding a new lint to `soroban-cost-linter`.
> [`docs/custom_lint_guide.md`](docs/custom_lint_guide.md) is only a pointer to this page.

This guide explains how to add a custom Dylint lint to `soroban-cost-linter`. It assumes that you know Rust, but have not previously worked with `rustc_hir`, compiler lint passes, or Clippy's helper APIs.

## Before you start

A lint in this repository should identify a structural Soroban cost problem: something that is expensive because of the shape of the code, rather than because of a particular runtime input. Before writing code:

1. Read the [scope boundary](docs/scope_boundary.md) to make sure the proposed lint belongs in this project and does not duplicate a Clippy lint.
2. Choose the relevant [lint category](docs/lint_categories.md) for the pattern — every lint is assigned one of the five `LintCategory` values defined in `soroban_cost_lints/src/lib.rs`.
3. Search the existing lints for similar traversal and matching logic.
4. Decide whether the pattern is easiest to recognize in the AST or HIR.

AST is close to the source syntax and is useful for simple syntactic patterns. HIR has resolved names, types, and expressions, so it is usually the better choice when the lint needs to distinguish a particular method, type, loop, or enclosing context.

Install the repository's Dylint tools before running the examples:

```bash
cargo install cargo-dylint dylint-link --version "^6.0.1"
```

Use the pinned toolchain from the repository. Compiler internals are tightly coupled to the Rust version, so changing toolchains while developing a lint can produce confusing errors.

## 1. Understand how a lint is put together

All lint names and their registration live in **one file**: [`soroban_cost_lints/src/lib.rs`](soroban_cost_lints/src/lib.rs). There is **no** `soroban_cost_lints/src/lints/` directory, and UI fixtures do **not** live in `tests/ui/`.

Each lint needs three things, all of which live in `lib.rs` (or, for the pass implementation, in a sibling module file that `lib.rs` declares):

| Piece | Where | What happens if it is missing |
| --- | --- | --- |
| `declare_lint!` block | `lib.rs` | `rustc` does not know the lint name, so it cannot be registered, referenced in `#[allow]`, or configured in `budget.toml`. |
| `LINT_METADATA` row | `lib.rs` (`LINT_METADATA` static, keyed by the snake_case lint name) | The `cargo-cost-lint` CLI does not list or document the lint. It still runs, but is invisible to users and `generate-lint-docs`. |
| Pass implementation + a line in the `dylint_lint_impl!` list | implementation in a module file (or inlined), declaration in `lib.rs`'s `dylint_lint_impl!` list | **The lint silently never runs.** Dylint only loads passes listed below `SorobanCostLints` in the `dylint_lint_impl!` macro at the bottom of `lib.rs`. |

Miss any one of the three and the lint does not work as intended — in particular, forgetting the `dylint_lint_impl!` entry produces no error at all, just a lint that never fires.

There are two accepted layouts, both present in the tree today:

- **Inline in `lib.rs`.** The majority of lints are declared, implemented, and registered entirely inside `lib.rs` (see the `declare_lint!` blocks and the `dylint_lint_impl!` list, which reference two dozen lints with no separate file).
- **Sibling module file.** Newer lints declare in `lib.rs` and implement the pass in a small module file. For example `ledger_context_read_in_loop` is declared in `lib.rs` and implemented in [`soroban_cost_lints/src/ledger_context_read_in_loop.rs`](soroban_cost_lints/src/ledger_context_read_in_loop.rs); its `mod ledger_context_read_in_loop;` declaration is at the top of `lib.rs`. Other examples: `redundant_require_auth.rs`, `discarded_storage_read.rs`, `option_wrapping_in_storage.rs`.

For a new lint, a sibling module file keeps `lib.rs` readable once you have many passes. Start from the closest existing module and copy its structure.

## 2. Walkthrough: `ledger_context_read_in_loop` (a real lint)

To make this concrete, this section walks through `ledger_context_read_in_loop`, a small, fully-shipped lint that already exists in the tree. It fires when a ledger-context value (`env.ledger().sequence()`, `.timestamp()`, `.network_id()`, or `.protocol_version()`) is read inside a loop, even though that value cannot change during a single invocation.

### 2a. Declare the lint in `lib.rs`

A lint reaches `rustc` through a `declare_lint!` block. `LEDGER_CONTEXT_READ_IN_LOOP` is declared with the other lints near the top of `lib.rs`:

```rust
rustc_lint::declare_lint! {
    pub LEDGER_CONTEXT_READ_IN_LOOP,
    Warn,
    "reads a ledger context value inside a loop when it cannot change during the invocation"
}
```

### 2b. Implement the pass

The pass decides *when* the lint fires. For `ledger_context_read_in_loop` the implementation lives in `soroban_cost_lints/src/ledger_context_read_in_loop.rs`. It detects the enclosing loop with `clippy_utils::get_enclosing_loop_or_multi_call_closure` rather than hand-rolling a parent walk, which is exactly the "use `clippy_utils` instead of rebuilding compiler logic" advice in this guide:

```rust
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::get_enclosing_loop_or_multi_call_closure;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::LEDGER_CONTEXT_READ_IN_LOOP;

declare_lint_pass!(LedgerContextReadInLoop => [LEDGER_CONTEXT_READ_IN_LOOP]);

const LEDGER_READ_METHODS: &[&str] = &["sequence", "timestamp", "network_id", "protocol_version"];

impl<'tcx> LateLintPass<'tcx> for LedgerContextReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        if let rustc_hir::ExprKind::MethodCall(path, receiver, _args, _) = expr.kind {
            let method_name = path.ident.as_str();
            if !LEDGER_READ_METHODS.contains(&method_name) {
                return;
            }
            if !is_ledger_receiver(cx, receiver) {
                return;
            }
            if get_enclosing_loop_or_multi_call_closure(cx, expr).is_some() {
                let help = format!(
                    "ledger context values ({method_name}) are invariant during a single \
                     invocation; hoist this read outside the loop to avoid repeated host calls"
                );
                span_lint_and_help(
                    cx,
                    LEDGER_CONTEXT_READ_IN_LOOP,
                    expr.span,
                    &format!(
                        "reading ledger context `{method_name}` inside a loop — the value \
                         cannot change during this invocation"
                    ),
                    None,
                    &help,
                );
            }
        }
    }
}

fn is_ledger_receiver(cx: &LateContext<'_>, expr: &Expr<'_>) -> bool {
    if let rustc_hir::ExprKind::MethodCall(path, receiver, args, _) = expr.kind {
        if path.ident.as_str() == "ledger" && args.is_empty() {
            let ty = cx.typeck_results().expr_ty(receiver);
            let ty_str = format!("{:?}", ty);
            return ty_str.contains("Env");
        }
    }
    false
}
```

Because `lib.rs` declares `LEDGER_CONTEXT_READ_IN_LOOP` and this module references it via `use crate::LEDGER_CONTEXT_READ_IN_LOOP`, the lint-pass macro (`declare_lint_pass!`) knows which lint this pass drives.

### 2c. Register all three coordinated edits in `lib.rs`

The first edit is the `declare_lint!` block (2a). The remaining two are:

**Edit 2 — a `LINT_METADATA` row.** The CLI and `generate-lint-docs` read this static. The name must exactly match the snake_case lint name. The `ledger_context_read_in_loop` row is:

```rust
LintMeta {
    name: "ledger_context_read_in_loop",
    category: LintCategory::Compute,
    description: "Reads a ledger context value (sequence, timestamp, network_id) inside a loop",
    rationale: "Ledger context values are invariant during a single invocation; reading them in a loop performs repeated host calls for the same value.",
},
```

**Edit 3 — a line in the `dylint_lint_impl!` list.** The pass is loaded by Dylint only if its lint name appears in the list under `SorobanCostLints` at the bottom of `lib.rs`:

```rust
dylint_lint_impl! {
    SorobanCostLints,
    [
        // ... existing lint names ...
        LEDGER_CONTEXT_READ_IN_LOOP,
        // ... more lint names ...
    ]
}
```

If you use a sibling module file, also add its `mod <module_name>;` declaration at the top of `lib.rs`. Then build the dynamic library from the crate directory so the local Dylint configuration is applied:

```bash
cd soroban_cost_lints
cargo build
```

If the lint is not discovered, check the module declaration, the `dylint_lint_impl!` entry, and the `LINT_METADATA` row before investigating the implementation — a missing `dylint_lint_impl!` line is the classic cause of a lint that "should work but never fires".

## 3. Write a UI fixture

UI tests verify both that a lint fires and that its diagnostic is stable. The source fixture and its expected output belong in **`soroban_cost_lints/ui/`**, not `tests/ui/`:

```text
soroban_cost_lints/ui/ledger_context_read_in_loop.rs
soroban_cost_lints/ui/ledger_context_read_in_loop.stderr
```

The `.rs` fixture is a small, self-contained program that stubs the Soroban SDK surface the lint needs. For `ledger_context_read_in_loop` that is an `Env` with a `ledger()` accessor:

```rust
pub mod soroban_sdk {
    pub struct Env;
    impl Env {
        pub fn ledger(&self) -> ledger::Ledger {
            ledger::Ledger
        }
    }

    pub mod ledger {
        pub struct Ledger;
        impl Ledger {
            pub fn sequence(&self) -> u32 { 0 }
            pub fn timestamp(&self) -> u64 { 0 }
            pub fn network_id(&self) -> [u8; 32] { [0u8; 32] }
        }
    }
}

use soroban_sdk::Env;
```

The fixture must contain at least:

- one intentionally bad case that must trigger the lint;
- one valid case that must not trigger it;
- boundary cases such as a nested block, closure, or different loop form when those affect the analysis;
- a `#[allow(<lint_name>)]` suppression case to confirm the lint can be turned off.

`ledger_context_read_in_loop.rs` covers all of these: reads inside `for`, `while`, `loop`, and a closure each warn, while the same reads outside a loop, a hoisted read, and an `#[allow(ledger_context_read_in_loop)]` function stay silent.

The `.stderr` file is the blessed expected compiler output, generated by the UI test harness. Run the workspace UI tests to compare and, when a fixture is new or a diagnostic intentionally changes, re-bless:

```bash
cargo test --workspace
BLESS=1 cargo test --workspace
```

Review every changed `.stderr` file after blessing. Never bless output merely to make a failing test pass: confirm that the changed warning, span, and suggestion are correct. The absence of a diagnostic for the non-triggering cases is part of the expected behavior, even though it does not appear as a separate line in `.stderr`.

## 4. Use `clippy_utils` instead of rebuilding compiler logic

`clippy_utils` contains small, well-tested helpers for common HIR and type-analysis tasks. Search its documentation and the Clippy source before writing your own parent traversal or type matching code. The version in this repository is pinned to match the Rust toolchain.

Useful categories of helpers include:

- **Loop and closure context:** `clippy_utils::get_enclosing_loop_or_multi_call_closure` (used above) finds the enclosing loop or a closure that may be called multiple times, returning `Option<&Expr>`. This is preferable to manually walking parents and accidentally missing a closure boundary. Confirm the exact signature against the pinned `clippy_utils` before relying on it (helper names and paths have changed across revisions — `clippy_utils::ops::is_inside_loop` no longer exists at the pinned rev).
- **Expression and path inspection:** helpers for extracting call arguments, method names, paths, and constants make matching less dependent on the exact HIR layout.
- **Type checks:** use Clippy's type utilities to determine whether an expression has the expected type or implements the relevant trait instead of comparing source text.
- **Source spans and diagnostics:** use `clippy_utils::diagnostics::span_lint`, `span_lint_and_help`, or `span_lint_and_sugg` to keep messages and highlighted spans consistent. The `cx.lint(...)` two-argument call from the old guide does **not** exist — the repository's diagnostics go through `clippy_utils::diagnostics`.
- **Parent and body traversal:** use established HIR traversal utilities when you need to inspect an enclosing body, statement, or expression.

A conceptually similar loop-sensitive check for a different pattern follows the same shape: match the relevant `ExprKind`, confirm the receiver/type, then gate the diagnostic behind `get_enclosing_loop_or_multi_call_closure(...).is_some()` so the check is precise rather than "any statement ancestor".

Function signatures can change with the pinned compiler and `clippy_utils` revision. Use rust-analyzer, `cargo doc`, or the existing call sites in the repository to confirm the current signature before copying an example. The important principle is to use the helper's semantic result rather than assuming that every parent block represents a loop.

For method calls, inspect the receiver and arguments separately. When possible, use type information or the resolved definition rather than assuming that every method named `insert`, `set`, or `clone` belongs to the Soroban type of interest. Existing lints in this repository are the best examples of how Soroban SDK paths are recognized.

Do not report the same source operation more than once when several visitor callbacks can reach it. Also consider nested loops, closures, `break`, and `continue` when the cost claim depends on loop context.

## 5. Run the complete checks

While iterating, build and run the focused UI tests first. Before opening a pull request, run all required repository checks from the workspace root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p generate-lint-docs -- --check
```

To benchmark the runtime overhead of the linter on real contracts, run:

```bash
cargo bench -p cargo-cost-lint
```

Also exercise the command-line tool against a small Soroban fixture if the new lint depends on SDK types or on JSON output. Confirm that the lint can be enabled and disabled with standard attributes and that its name appears correctly in `budget.toml` configuration.

## 6. Document the new lint

A user-facing lint needs a page under [`docs/lints`](docs/lints/README.md). Include:

- the pattern that is detected;
- why the pattern increases Soroban resource usage;
- a triggering example;
- a recommended rewrite;
- intentional cases that may need `#[allow(lint_name)]`;
- the default severity and cost category.

`docs/lints/README.md` and `docs/lints/lint-registry.json` are generated by `tools/generate-lint-docs` from `lib.rs`'s `LINT_METADATA` — do not edit them by hand. After updating the `LINT_METADATA` row, regenerate them:

```bash
cargo run -p generate-lint-docs
```

Then review the diff on `docs/lints/` and update the top-level `README.md` and `soroban_cost_lints/README.md` lint tables if appropriate. Keep the declared name, metadata name, documentation links, and examples consistent.

Finally, follow the contribution process in [`CONTRIBUTING.md`](CONTRIBUTING.md). The pull request should explain the cost model motivation, summarize the false-positive safeguards, include the UI coverage, and include `Closes #[this issue]` in the PR description as required by the project issue template.
