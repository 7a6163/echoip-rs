use std::net::IpAddr;
use std::num::NonZeroUsize;

use lru::LruCache;

use crate::response::Response;

pub struct Cache {
    inner: LruCache<IpAddr, Response>,
    evictions: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CacheStats {
    pub size: usize,
    pub capacity: usize,
    pub evictions: u64,
}

impl Cache {
    pub fn new(capacity: usize) -> Self {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        Self {
            inner: LruCache::new(cap),
            evictions: 0,
        }
    }

    pub fn set(&mut self, ip: IpAddr, response: Response) {
        if self.capacity() == 0 {
            return;
        }

        if self.inner.len() == self.inner.cap().get() && self.inner.peek(&ip).is_none() {
            self.evictions += 1;
        }

        self.inner.push(ip, response);
    }

    pub fn get(&mut self, ip: IpAddr) -> Option<Response> {
        self.inner.get(&ip).cloned()
    }

    pub fn resize(&mut self, capacity: usize) {
        let cap = NonZeroUsize::new(capacity.max(1)).unwrap();
        self.inner.resize(cap);
        self.evictions = 0;
    }

    pub fn capacity(&self) -> usize {
        self.inner.cap().get()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.inner.len(),
            capacity: self.inner.cap().get(),
            evictions: self.evictions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_bigint::BigUint;

    fn make_response(ip: IpAddr) -> Response {
        Response {
            ip,
            ip_decimal: BigUint::from(0u32),
            country: String::new(),
            country_iso: String::new(),
            country_eu: false,
            region_name: String::new(),
            region_code: String::new(),
            metro_code: 0,
            zip_code: String::new(),
            city: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            time_zone: String::new(),
            asn: String::new(),
            asn_org: String::new(),
            hostname: String::new(),
            user_agent: None,
        }
    }

    #[test]
    fn test_cache_capacity() {
        let tests = vec![
            (1, 0, 0, 0u64),
            (1, 2, 1, 0),
            (2, 2, 2, 0),
            (3, 2, 2, 1),
            (10, 5, 5, 5),
        ];

        for (add_count, capacity, expected_size, expected_evictions) in tests {
            let mut cache = Cache::new(capacity);
            let mut ips = Vec::new();
            for i in 0..add_count {
                let ip: IpAddr = format!("192.0.2.{i}").parse().unwrap();
                ips.push(ip);
                cache.set(ip, make_response(ip));
            }

            let actual_size = if capacity == 0 { 0 } else { cache.inner.len() };
            assert_eq!(
                actual_size, expected_size,
                "size mismatch for add={add_count} cap={capacity}"
            );
            assert_eq!(
                cache.evictions, expected_evictions,
                "evictions mismatch for add={add_count} cap={capacity}"
            );

            if capacity > 0 && add_count > capacity && capacity == expected_size {
                let last = ips[add_count - 1];
                assert!(cache.get(last).is_some(), "last added should be in cache");
                let first = ips[0];
                assert!(cache.get(first).is_none(), "first added should be evicted");
            }
        }
    }

    #[test]
    fn test_cache_duplicate() {
        let mut cache = Cache::new(10);
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        cache.set(ip, make_response(ip));
        cache.set(ip, make_response(ip));
        assert_eq!(cache.inner.len(), 1);
    }

    #[test]
    fn test_cache_resize() {
        let mut cache = Cache::new(10);
        for i in 1..=20u8 {
            let ip: IpAddr = format!("192.0.2.{i}").parse().unwrap();
            cache.set(ip, make_response(ip));
        }
        assert_eq!(cache.inner.len(), 10);
        assert_eq!(cache.evictions, 10);

        cache.resize(5);
        assert_eq!(cache.evictions, 0);

        let ip: IpAddr = "192.0.2.42".parse().unwrap();
        cache.set(ip, make_response(ip));
        assert_eq!(cache.inner.len(), 5);
    }

    #[test]
    fn test_lru_eviction_order() {
        let mut cache = Cache::new(3);
        let ip1: IpAddr = "192.0.2.1".parse().unwrap();
        let ip2: IpAddr = "192.0.2.2".parse().unwrap();
        let ip3: IpAddr = "192.0.2.3".parse().unwrap();
        let ip4: IpAddr = "192.0.2.4".parse().unwrap();

        cache.set(ip1, make_response(ip1));
        cache.set(ip2, make_response(ip2));
        cache.set(ip3, make_response(ip3));

        // Access ip1, making ip2 the least recently used
        let _ = cache.get(ip1);

        // Adding ip4 should evict ip2 (LRU), not ip1
        cache.set(ip4, make_response(ip4));

        assert!(cache.get(ip1).is_some(), "ip1 was accessed recently, should survive");
        assert!(cache.get(ip2).is_none(), "ip2 was LRU, should be evicted");
        assert!(cache.get(ip3).is_some(), "ip3 should survive");
        assert!(cache.get(ip4).is_some(), "ip4 was just added");
    }
}
