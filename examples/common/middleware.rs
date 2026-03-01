use easy_rmq::Result;

// Example: Logging middleware (function-based)
pub fn logging(_payload: &[u8], result: &Result<()>) -> Result<()> {
    match result {
        Ok(_) => tracing::info!("✓ Message processed successfully"),
        Err(e) => tracing::error!("✗ Message processing failed: {:?}", e),
    }
    Ok(())
}

// Example: Metrics middleware (function-based with execution time)
pub fn metrics(_payload: &[u8], result: &Result<()>) -> Result<()> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SUCCESS_COUNT: AtomicU64 = AtomicU64::new(0);
    static ERROR_COUNT: AtomicU64 = AtomicU64::new(0);
    static TOTAL_TIME_US: AtomicU64 = AtomicU64::new(0);

    match result {
        Ok(_) => {
            let count = SUCCESS_COUNT.fetch_add(1, Ordering::Relaxed) + 1;

            // Get execution time from middleware blanket impl
            if let Some(elapsed_us) = easy_rmq::get_execution_time_us() {
                let total = TOTAL_TIME_US.fetch_add(elapsed_us, Ordering::Relaxed) + elapsed_us;
                let avg_us = total / count;
                tracing::info!(
                    "📊 Metrics: {} ✓ | Avg: {}μs | Last: {}μs",
                    count,
                    avg_us,
                    elapsed_us
                );
            } else {
                tracing::info!("📊 Metrics: {} messages processed successfully", count);
            }
        }
        Err(_) => {
            let count = ERROR_COUNT.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::warn!("✗ Metrics: {} messages failed", count);
        }
    }
    Ok(())
}

// Example: Tracing middleware (function-based)
pub fn tracing(_payload: &[u8], result: &Result<()>) -> Result<()> {
    match result {
        Ok(_) => tracing::debug!("Message processed"),
        Err(e) => tracing::warn!("Message failed: {:?}", e),
    }
    Ok(())
}
