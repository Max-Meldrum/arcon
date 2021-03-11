use super::*;

pub struct TSSImpl {
    identifier: String,
    // plan..
}
impl TSSImpl {
    pub fn new(identifier: String) -> Self {
        Self {
            identifier,
        }
    }
}

impl TSS for TSSImpl {
    fn selection(self, _: SelectionExpr) -> Arc<dyn TSS> {
        Arc::new(self)
    }
    fn join(self, _: Arc<dyn TSS>) -> Arc<dyn TSS> {
        Arc::new(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::prelude::*;
    use std::sync::Arc;

    #[test]
    fn tss() {
        let tss: Arc<dyn TSS> = Arc::new(TSSImpl::new("events"));
        let date = NaiveDate::from_ymd(2020, 7, 8).and_hms(9, 10, 11);
        // Define a TSS object that represents events state when time has exceeded date
        let events_tss = tss.selection(when_time_gt(date));
    }
}
