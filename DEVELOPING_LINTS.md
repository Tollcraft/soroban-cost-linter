# Developing Lints

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

## 1. Scaffold the lint module

Lint implementations live in `soroban_cost_lints/src`. Create a module for the new lint, using a lowercase snake-case name that matches the public lint name. For example, a lint named `storage_read_in_loop` would start in:

```text
soroban_cost_lints/src/storage_read_in_loop.rs
```

Begin with the lint declaration and a pass. The exact trait and helper macro used by the repository can be copied from the closest existing lint. A typical Dylint lint has these parts:

```rust
use clippy_utils::diagnostics::span_lint;
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::{declare_lint, declare_lint_pass};

// The lint name is part of the public interface. It is used by Rust attributes
// and by budget.toml, so do not rename it after release.
declare_lint! {
    pub STORAGE_READ_IN_LOOP,
    Warn,
    "storage reads inside loop bodies"
}

declare_lint_pass!(StorageReadInLoop => [STORAGE_READ_IN_LOOP]);

impl<'tcx> LateLintPass<'tcx> for StorageReadInLoop {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // Match the relevant HIR expression and emit a diagnostic when the
        // enclosing context proves that it is inside a loop.
        let _ = (cx, expr);
    }
}
```

The names in the example are illustrative. Follow the imports, declaration style, and pass type used by the existing module that most closely resembles your pattern. In particular, do not mix an AST pass and a HIR pass just because both are available: choose the representation that makes the check precise and maintainable.

### Prefer precise matching

Compiler HIR is a tree of expressions, statements, patterns, and bodies. A useful workflow is:

1. Start at the callback that receives the node you need, such as `check_expr`.
2. Match only the expression kinds relevant to the lint.
3. Resolve names or definitions when matching methods. Textual method names alone can produce false positives for unrelated types.
4. Walk parents or use a helper to establish context, such as whether the expression is in a loop.
5. Emit one diagnostic for the smallest useful source span.

For method calls, inspect the receiver and arguments separately. When possible, use type information or the resolved definition rather than assuming that every method named `insert`, `set`, or `clone` belongs to the Soroban type of interest. Existing lints in this repository are the best examples of how Soroban SDK paths are recognized.

Do not report the same source operation more than once when several visitor callbacks can reach it. Also consider nested loops, closures, `break`, and `continue` when the cost claim depends on loop context.

## 2. Register the lint in `lib.rs`

Declaring a lint in its module is not enough: Dylint must export it from the library. Open `soroban_cost_lints/src/lib.rs` and follow the existing registration pattern.

Add the module declaration:

```rust
mod storage_read_in_loop;
```

Then add the pass to the exported lint list or registration macro used by the file. For example, if the library uses a combined pass, add the new pass alongside the existing passes:

```rust
pub struct SorobanCostLints;

// Add StorageReadInLoop to the list used by the repository's registration.
```

Use the actual macro or list already present in `lib.rs`; do not create a second registration mechanism. The module, lint declaration, and exported pass all need to be present for the lint to load through Dylint.

The repository also maintains lint metadata in `LINT_METADATA`. Add an entry there with the lint name, cost category, and user-facing description. Keep the metadata name exactly synchronized with the declared lint name. This registry is used by the command-line tool and configuration validation.

After registration, build the dynamic library from its crate directory so the local Dylint configuration is applied:

```bash
cd soroban_cost_lints
cargo build
```

If the lint is not discovered, first check the module declaration, the exported pass list, the metadata entry, and `DYLINT_LIBRARY_PATH` before investigating the implementation.

## 3. Write a UI fixture

UI tests verify both that a lint fires and that its diagnostic is stable. The source fixture belongs in the lint crate's UI test directory, alongside the existing `main.rs` harness:

```text
soroban_cost_lints/ui/storage_read_in_loop.rs
soroban_cost_lints/ui/storage_read_in_loop.stderr
```

The `.rs` fixture should contain a small, self-contained example. Include at least:

- one intentionally bad case that must trigger the lint;
- one valid case that must not trigger it;
- boundary cases such as a nested block, closure, or different loop form when those affect the analysis;
- enough surrounding code for the diagnostic span to be meaningful, but no unrelated application logic.

For example:

```rust
fn read_in_loop(env: soroban_sdk::Env, key: u32) {
    for _ in 0..3 {
        let _ = env.storage().instance().get::<u32, u32>(&key);
    }
}

fn read_once(env: soroban_sdk::Env, key: u32) {
    let _ = env.storage().instance().get::<u32, u32>(&key);
}
```

Use the SDK fixture and imports required by the pattern under test. A fixture should compile far enough for rustc to run the lint; avoid deliberately introducing unrelated compiler errors.

The `.stderr` file is the expected compiler output generated by `dylint_testing`. It records the lint name, severity, message, source location, highlighted span, and any help text. Do not hand-edit line numbers unless there is a specific reason; regenerate the expected output after intentional diagnostic changes.

The existing UI harness in `soroban_cost_lints/ui/main.rs` defines how these fixtures are discovered. Run the UI test target used by that harness from the lint crate. A typical invocation is:

```bash
cd soroban_cost_lints
cargo test --test ui
```

If the harness is configured as a different test target, use the target name shown in `soroban_cost_lints/Cargo.toml`. `dylint_testing` compiles the fixture and compares rustc's output with the adjacent `.stderr` file.

### Updating expected output

When a fixture is new or a diagnostic intentionally changes, run the UI tests in bless mode using the convention supported by the repository's Dylint harness. Dylint projects commonly use:

```bash
cd soroban_cost_lints
TRYBUILD=overwrite cargo test --test ui
```

Some harness versions use `BLESS=1` instead. Check the existing test configuration and output from the harness rather than guessing. Review every changed `.stderr` file after blessing. Never bless output merely to make a failing test pass: confirm that the changed warning, span, and suggestion are correct.

A good UI test suite should also cover non-triggering examples. The absence of a diagnostic is part of the expected behavior, even though it does not appear as a separate line in `.stderr`.

## 4. Use `clippy_utils` instead of rebuilding compiler logic

`clippy_utils` contains small, well-tested helpers for common HIR and type-analysis tasks. Search its documentation and the Clippy source before writing your own parent traversal or type matching code. The version in this repository is pinned to match the Rust toolchain.

Useful categories of helpers include:

- **Loop and closure context:** `get_enclosing_loop_or_multi_call_closure` finds the enclosing loop or a closure that may be called multiple times. This is useful when a host or storage operation is expensive because it can execute repeatedly. It is preferable to manually walking parents and accidentally missing a closure boundary.
- **Expression and path inspection:** helpers for extracting call arguments, method names, paths, and constants make matching less dependent on the exact HIR layout.
- **Type checks:** use Clippy's type utilities to determine whether an expression has the expected type or implements the relevant trait instead of comparing source text.
- **Source spans and diagnostics:** use `span_lint`, `span_lint_and_help`, or the repository's existing diagnostic helpers to keep messages and highlighted spans consistent.
- **Parent and body traversal:** use established HIR traversal utilities when you need to inspect an enclosing body, statement, or expression.

For example, a loop-sensitive check conceptually follows this shape:

```rust
if let Some(enclosing) = clippy_utils::loops::get_enclosing_loop_or_multi_call_closure(
    cx.tcx,
    expr.hir_id,
) {
    // Confirm that `expr` is the operation this lint is about, then diagnose it.
    let _ = enclosing;
}
```

Function signatures can change with the pinned compiler and `clippy_utils` revision. Use rust-analyzer, `cargo doc`, or the existing call sites in the repository to confirm the current signature before copying an example. The important principle is to use the helper's semantic result rather than assuming that every parent block represents a loop.

## 5. Run the complete checks

While iterating, build and run the focused UI tests first. Before opening a pull request, run all required repository checks from the workspace root:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
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

Add the page to `docs/lints/README.md`, update the lint list in `README.md`, and update `soroban_cost_lints/README.md` if appropriate. Keep the declared name, metadata name, documentation links, and examples consistent.

Finally, follow the contribution process in [`CONTRIBUTING.md`](CONTRIBUTING.md). The pull request should explain the cost model motivation, summarize the false-positive safeguards, include the UI coverage, and include `Closes #[this issue]` in the PR description as required by the project issue template.
