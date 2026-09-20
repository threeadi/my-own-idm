use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A thread-safe, smooth Token Bucket Rate Limiter for throttling download streams.
/// Can be shared across multiple worker threads or tasks using Arc.
#[derive(Debug)]
pub struct TokenBucketRateLimiter {
    limit_bps: Arc<AtomicU64>,
    state: parking_lot::Mutex<LimiterState>,
}

#[derive(Debug)]
struct LimiterState {
    last_checked: Instant,
    tokens: f64,
}

impl TokenBucketRateLimiter {
    /// Creates a new rate limiter with the specified bytes per second (bps).
    /// If limit_bps is 0, rate limiting is disabled (unlimited).
    pub fn new(limit_bps: u64) -> Self {
        let now = Instant::now();
        Self {
            limit_bps: Arc::new(AtomicU64::new(limit_bps)),
            state: parking_lot::Mutex::new(LimiterState {
                last_checked: now,
                tokens: limit_bps as f64, // Start with a full second bucket
            }),
        }
    }

    /// Dynamically changes the rate limit in bytes per second without restarting downloads.
    pub fn set_limit_bps(&self, new_limit: u64) {
        let mut state = self.state.lock();
        state.last_checked = Instant::now();
        state.tokens = (new_limit as f64).min(state.tokens);
        self.limit_bps.store(new_limit, Ordering::Relaxed);
    }

    /// Retrieves the current rate limit in bytes per second.
    pub fn get_limit_bps(&self) -> u64 {
        self.limit_bps.load(Ordering::Relaxed)
    }

    /// Acquires permission to transmit `bytes`. If tokens are insufficient,
    /// asynchronously sleeps for the required duration without blocking OS threads.
    pub async fn acquire(&self, bytes: usize) {
        if bytes == 0 {
            return;
        }

        let limit = self.limit_bps.load(Ordering::Relaxed);
        if limit == 0 {
            return; // Unlimited throughput
        }

        let wait_duration = {
            let mut state = self.state.lock();
            let now = Instant::now();

            let elapsed = if now > state.last_checked {
                now.duration_since(state.last_checked).as_secs_f64()
            } else {
                0.0
            };

            // Max bucket capacity is 0.5s burst or at least 128 KB
            let max_capacity = (limit as f64 * 0.5).max(128.0 * 1024.0);
            state.tokens = (state.tokens + elapsed * limit as f64).min(max_capacity);
            state.last_checked = now;

            let bytes_f64 = bytes as f64;
            if state.tokens >= bytes_f64 {
                state.tokens -= bytes_f64;
                None
            } else {
                let needed = bytes_f64 - state.tokens;
                let wait_secs = needed / (limit as f64);
                state.tokens = 0.0;
                state.last_checked = now + Duration::from_secs_f64(wait_secs);
                Some(Duration::from_secs_f64(wait_secs))
            }
        };

        if let Some(dur) = wait_duration {
            tokio::time::sleep(dur).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_unlimited() {
        let limiter = TokenBucketRateLimiter::new(0);
        assert_eq!(limiter.get_limit_bps(), 0);

        let start = Instant::now();
        limiter.acquire(1024 * 1024).await;
        limiter.acquire(0).await;
        assert!(start.elapsed() < Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_rate_limiter_dynamic_update() {
        let limiter = TokenBucketRateLimiter::new(1024 * 1024);
        assert_eq!(limiter.get_limit_bps(), 1024 * 1024);

        limiter.set_limit_bps(2 * 1024 * 1024);
        assert_eq!(limiter.get_limit_bps(), 2 * 1024 * 1024);

        limiter.set_limit_bps(0);
        assert_eq!(limiter.get_limit_bps(), 0);
    }

    #[tokio::test]
    async fn test_rate_limiter_pacing() {
        // Limit to 10 KB/s (10,240 B/s)
        let limiter = TokenBucketRateLimiter::new(10_240);
        
        // Drain initial bucket
        limiter.acquire(10_240).await;

        // Next acquire of 5,120 bytes should take ~500ms (0.5s)
        let start = Instant::now();
        limiter.acquire(5_120).await;
        let elapsed = start.elapsed();

        assert!(
            elapsed >= Duration::from_millis(400),
            "Expected elapsed >= 400ms, got {:?}",
            elapsed
        );
    }
}
