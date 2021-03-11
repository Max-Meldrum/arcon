use chrono::prelude::*;
use std::sync::Arc;

mod tss_impl;

include!(concat!(env!("OUT_DIR"), "/arcon_tss.rs"));

pub fn when_time_gt(date: NaiveDateTime) -> SelectionExpr {
    let ts = date.timestamp() as u64;
    let time_gt_expr = TimeGtExpr { timestamp: ts };
    let when_expr = WhenExpr {
        expr: Some(when_expr::Expr::TimeGtExpr(time_gt_expr)),
    };

    SelectionExpr {
        expr: Some(selection_expr::Expr::WhenExpr(when_expr)),
    }
}
pub fn latest() -> SelectionExpr {
    SelectionExpr {
        expr: Some(selection_expr::Expr::LatestExpr(LatestExpr {})),
    }
}

pub fn next() -> SelectionExpr {
    SelectionExpr {
        expr: Some(selection_expr::Expr::NextExpr(NextExpr {})),
    }
}

/// Temporal Stream State
///
/// This trait defines all the methods a TSS must implement
#[async_trait]
pub trait TSS: Send + Sync {
    /// Define the stream state version
    fn selection(self, expr: SelectionExpr) -> Arc<dyn TSS>;
    /// Join one TSS with another one
    fn join(self, other: Arc<dyn TSS>) -> Arc<dyn TSS>;
}
