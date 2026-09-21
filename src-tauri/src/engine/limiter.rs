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
        let limit_f64 = limit_bps as f64;
        let initial_tokens = (limit_f64 * 0.25).max(1024.0);
        Self {
            limit_bps: Arc::new(AtomicU64::new(limit_bps)),
            state: parking_lot::Mutex::new(LimiterState {
                last_checked: now,
                tokens: initial_tokens,
            }),
        }
    }

    /// Dynamically changes the rate limit in bytes per second without restarting downloads.
    pub fn set_limit_bps(&self, new_limit: u64) {
        let mut state = self.state.lock();
        state.last_checked = Instant::now();
        let max_capacity = (new_limit as f64 * 0.25).max(1024.0);
        state.tokens = state.tokens.min(max_capacity);
        self.limit_bps.store(new_limit, Ordering::Relaxed);
    }

    /// Retrieves the current rate limit in bytes per second.
    pub fn get_limit_bps(&self) -> u64 {
        self.limit_bps.load(Ordering::Relaxed)
    }

    /// Acquires permission to transmit `bytes`. If tokens are insufficient,
    /// asynchronously sleeps for the required duration without blocking OS threads.
    /// Uses a token-debt queuing model to ensure multiple concurrent workers/threads
    /// aggregate strictly to the target limit without race conditions.
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
            let limit_f64 = limit as f64;

            let elapsed = if now > state.last_checked {
                now.duration_since(state.last_checked).as_secs_f64()
            } else {
                0.0
            };
            state.last_checked = now;

            // Replenish tokens, capped to at most 0.25s of allowance
            let max_capacity = (limit_f64 * 0.25).max(1024.0);
            state.tokens = (state.tokens + elapsed * limit_f64).min(max_capacity);

            let bytes_f64 = bytes as f64;
            state.tokens -= bytes_f64;

            if state.tokens >= 0.0 {
                None
            } else {
                let debt = -state.tokens;
                let wait_secs = debt / limit_f64;
                // Cap accumulated debt delay to 2.0s to avoid indefinite stalls
                Some(Duration::from_secs_f64(wait_secs.min(2.0)))
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

    #[tokio::test]
    async fn test_concurrent_multi_thread_rate_limiting() {
        // 8 concurrent workers sharing a 200 KB/s (204,800 B/s) limiter
        // Total data across 8 workers = 8 * 25,600 = 204,800 bytes (1 full second worth of data)
        let limiter = Arc::new(TokenBucketRateLimiter::new(204_800));
        
        // Drain initial burst
        limiter.acquire(204_800).await;

        let start = Instant::now();
        let mut handles = Vec::new();
        for _ in 0..8 {
            let lim = limiter.clone();
            handles.push(tokio::spawn(async move {
                lim.acquire(25_600).await;
            }));
        }

        for h in handles {
            h.await.unwrap();
        }

        let elapsed = start.elapsed();
        // Since 8 workers concurrently requested 204,800 bytes after drain,
        // it must take at least 800ms (close to 1 second) across all 8 threads!
        assert!(
            elapsed >= Duration::from_millis(800),
            "Expected concurrent elapsed >= 800ms, got {:?}",
            elapsed
        );
    }
}
