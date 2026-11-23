use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::is_in_test;
use clippy_utils::macros::{FormatArgsStorage, root_macro_call_first_node};
use rustc_hir::Expr;
use rustc_lint::{LateContext, LateLintPass};
use rustc_session::impl_lint_pass;
use rustc_span::{ExpnId, sym};

use crate::FORMATTED_ASSERT_PAYLOAD;

const MSG: &str = "assert macro formats its failure message eagerly";

const HELP: &str = "a formatted assertion builds its message through core::fmt on every call; use a plain assert or panic_with_error!(env, Error::Variant) with a #[contracterror] enum instead";

pub struct FormattedAssertPayload {
    format_args: FormatArgsStorage,
    reported: Vec<ExpnId>,
}

impl_lint_pass!(FormattedAssertPayload => [FORMATTED_ASSERT_PAYLOAD]);

impl FormattedAssertPayload {
    pub fn new(format_args: FormatArgsStorage) -> Self {
        Self {
            format_args,
            reported: Vec::new(),
        }
    }
}

impl<'tcx> LateLintPass<'tcx> for FormattedAssertPayload {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        let Some(macro_call) = root_macro_call_first_node(cx, expr) else {
            return;
        };
        if !is_target_assert_macro(cx, macro_call.def_id)
            || self.reported.contains(&macro_call.expn)
            || is_in_test(cx.tcx, expr.hir_id)
        {
            return;
        }
        let Some(format_args) = self.format_args.get(cx, expr, macro_call.expn) else {
            return;
        };
        if format_args.arguments.all_args().is_empty() {
            return;
        }

        self.reported.push(macro_call.expn);
        span_lint_and_help(
            cx,
            FORMATTED_ASSERT_PAYLOAD,
            macro_call.span,
            MSG,
            None,
            HELP,
        );
    }
}

fn is_target_assert_macro(cx: &LateContext<'_>, def_id: rustc_span::def_id::DefId) -> bool {
    cx.tcx.is_diagnostic_item(sym::assert_macro, def_id)
        || cx.tcx.is_diagnostic_item(sym::debug_assert_macro, def_id)
        || cx.tcx.is_diagnostic_item(sym::assert_eq_macro, def_id)
        || cx.tcx.is_diagnostic_item(sym::assert_ne_macro, def_id)
}
