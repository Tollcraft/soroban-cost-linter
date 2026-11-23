//! `include_bytes_in_contract` — flags `include_bytes!` and `include_str!` data embedded in contract code.
//!
//! `include_bytes!("...")` and `include_str!("...")` splice the whole file into
//! the contract's data section, so the payload is paid for in Wasm bytecode size
//! and the deployment fee on every deploy, even when the invocation never reads it.

use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::macros::root_macro_call_first_node;
use rustc_hir as hir;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::declare_lint_pass;

use crate::INCLUDE_BYTES_IN_CONTRACT;

declare_lint_pass!(IncludeBytesInContract => [INCLUDE_BYTES_IN_CONTRACT]);

const INCLUDE_BYTES_IN_CONTRACT_HELP: &str =
    "load the payload from storage or pass it as an argument";

impl<'tcx> LateLintPass<'tcx> for IncludeBytesInContract {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx hir::Expr<'tcx>) {
        if let Some(macro_call) = root_macro_call_first_node(cx, expr) {
            let macro_name = cx.tcx.item_name(macro_call.def_id);
            let name_str = macro_name.as_str();
            if name_str != "include_bytes" && name_str != "include_str" {
                return;
            }

            let reason = format!("{name_str}! embeds the file in the Wasm data section");
            span_lint_and_help(
                cx,
                INCLUDE_BYTES_IN_CONTRACT,
                macro_call.span,
                reason,
                None,
                INCLUDE_BYTES_IN_CONTRACT_HELP,
            );
        }
    }
}
