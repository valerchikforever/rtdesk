//! Per-IP token buckets (SEC: T-32, T-40, AS-08 fair use — never a paywall).

use std::collections::HashMap;
use std::net::IpAddr;
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Connect,
    Register,
    Lookup,
}

impl Kind {
    /// (burst capacity, tokens per second)
    fn params(self) -> (f64, f64) {
        match self {
            Kind::Connect => (30.0, 1.0),
            Kind::Register => (10.0, 0.2),
            Kind::Lookup => (20.0, 1.0 / 3.0),
        }
    }
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

pub struct RateLimiter {
    buckets: HashMap<(IpAddr, Kind), Bucket>,
}

const MAX_ENTRIES: usize = 200_000;

impl RateLimiter {
    pub fn new() -> Self {
        Self { buckets: HashMap::new() }
    }

    pub fn allow(&mut self, ip: IpAddr, kind: Kind) -> bool {
        self.allow_at(ip, kind, Instant::now())
    }

    fn allow_at(&mut self, ip: IpAddr, kind: Kind, now: Instant) -> bool {
        if self.buckets.len() > MAX_ENTRIES {
            // Drop buckets that have fully refilled anyway.
            self.buckets.retain(|(_, k), b| {
                let (cap, rate) = k.params();
                b.tokens + now.duration_since(b.last).as_secs_f64() * rate < cap
            });
        }
        let (cap, rate) = kind.params();
        let b = self.buckets.entry((ip, kind)).or_insert(Bucket { tokens: cap, last: now });
        b.tokens = (b.tokens + now.duration_since(b.last).as_secs_f64() * rate).min(cap);
        b.last = now;
        if b.tokens >= 1.0 {
            b.tokens -= 1.0;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn burst_then_refill() {
        let mut l = RateLimiter::new();
        let ip: IpAddr = "1.2.3.4".parse().unwrap();
        let t = Instant::now();
        for _ in 0..20 {
            assert!(l.allow_at(ip, Kind::Lookup, t));
        }
        assert!(!l.allow_at(ip, Kind::Lookup, t));
        assert!(l.allow_at("5.6.7.8".parse().unwrap(), Kind::Lookup, t), "per IP");
        assert!(l.allow_at(ip, Kind::Lookup, t + Duration::from_secs(4)));
    }
}
