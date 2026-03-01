use crate::Result;
use lapin::types::FieldTable;

pub trait Middleware: Send + Sync {
    fn before(&self, payload: &[u8]) -> Result<()> {
        let _ = payload;
        Ok(())
    }

    fn after(&self, payload: &[u8], result: &Result<()>) -> Result<()> {
        let _ = payload;
        let _ = result;
        Ok(())
    }
}

// Global thread_local for execution time (shared across all)
mod timing {
    use std::cell::RefCell;

    thread_local! {
        pub(super) static START_TIME: RefCell<Option<u64>> = const { RefCell::new(None) };
        pub(super) static LAST_ELAPSED_US: RefCell<Option<u64>> = const { RefCell::new(None) };
    }
}

// Global thread_local for headers (shared across all)
mod headers {
    use lapin::types::FieldTable;
    use std::cell::RefCell;

    thread_local! {
        pub(super) static HEADERS: RefCell<Option<FieldTable>> = const { RefCell::new(None) };
    }
}

// Helper to get last execution time in microseconds
pub fn get_execution_time_us() -> Option<u64> {
    timing::LAST_ELAPSED_US.with(|elapsed| *elapsed.borrow())
}

// Helper to get current message headers
pub fn get_headers() -> Option<FieldTable> {
    headers::HEADERS.with(|h| h.borrow().clone())
}

// Helper to set headers (called by subscriber before processing)
pub(super) fn set_headers(headers: Option<FieldTable>) {
    headers::HEADERS.with(|h| *h.borrow_mut() = headers);
}

// Implement Middleware for any function with matching signature
impl<F> Middleware for F
where
    F: Fn(&[u8], &Result<()>) -> Result<()> + Copy + Send + Sync,
{
    fn before(&self, _payload: &[u8]) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        timing::START_TIME.with(|start| {
            *start.borrow_mut() = Some(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64,
            );
        });

        Ok(())
    }

    fn after(&self, payload: &[u8], result: &Result<()>) -> Result<()> {
        use std::time::{SystemTime, UNIX_EPOCH};

        timing::START_TIME.with(|start| {
            if let Some(start_us) = start.borrow_mut().take() {
                let end_us = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_micros() as u64;
                let elapsed_us = end_us.saturating_sub(start_us);

                timing::LAST_ELAPSED_US.with(|elapsed| {
                    *elapsed.borrow_mut() = Some(elapsed_us);
                });
            }
        });

        self(payload, result)
    }
}
