use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use fnv::FnvHasher;
use std::hash::{Hash, Hasher};

use crate::response::Response;

fn cache_key(ip: IpAddr) -> u64 {
    let mut hasher = FnvHasher::default();
    match ip {
        IpAddr::V4(v4) => v4.octets().hash(&mut hasher),
        IpAddr::V6(v6) => v6.octets().hash(&mut hasher),
    }
    hasher.finish()
}

struct CacheEntry {
    key: u64,
    #[allow(dead_code)]
    ip: IpAddr,
    response: Response,
}

pub struct Cache {
    capacity: usize,
    entries: HashMap<u64, usize>, // key -> index in values
    values: VecDeque<CacheEntry>,
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
        Self {
            capacity,
            entries: HashMap::new(),
            values: VecDeque::new(),
            evictions: 0,
        }
    }

    pub fn set(&mut self, ip: IpAddr, response: Response) {
        if self.capacity == 0 {
            return;
        }

        let k = cache_key(ip);

        // Evict if at or above capacity
        let min_evictions = (self.entries.len() + 1).saturating_sub(self.capacity);
        if min_evictions > 0 {
            for _ in 0..min_evictions {
                if let Some(entry) = self.values.pop_front() {
                    self.entries.remove(&entry.key);
                    self.evictions += 1;
                }
            }
            // Rebuild index after eviction
            self.rebuild_index();
        }

        // Remove existing entry for this key
        if let Some(idx) = self.entries.remove(&k) {
            self.values.remove(idx);
            self.rebuild_index();
        }

        let new_idx = self.values.len();
        self.values.push_back(CacheEntry {
            key: k,
            ip,
            response,
        });
        self.entries.insert(k, new_idx);
    }

    pub fn get(&self, ip: IpAddr) -> Option<Response> {
        let k = cache_key(ip);
        let idx = self.entries.get(&k)?;
        self.values.get(*idx).map(|e| e.response.clone())
    }

    pub fn resize(&mut self, capacity: usize) {
        self.capacity = capacity;
        self.evictions = 0;
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            size: self.entries.len(),
            capacity: self.capacity,
            evictions: self.evictions,
        }
    }

    fn rebuild_index(&mut self) {
        self.entries.clear();
        for (i, entry) in self.values.iter().enumerate() {
            self.entries.insert(entry.key, i);
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
            assert_eq!(
                cache.entries.len(),
                expected_size,
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
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.values.len(), 1);
    }

    #[test]
    fn test_cache_resize() {
        let mut cache = Cache::new(10);
        for i in 1..=20u8 {
            let ip: IpAddr = format!("192.0.2.{i}").parse().unwrap();
            cache.set(ip, make_response(ip));
        }
        assert_eq!(cache.entries.len(), 10);
        assert_eq!(cache.evictions, 10);

        cache.resize(5);
        assert_eq!(cache.evictions, 0);

        let ip: IpAddr = "192.0.2.42".parse().unwrap();
        cache.set(ip, make_response(ip));
        // After resize to 5, adding one more should evict down to 5
        assert_eq!(cache.entries.len(), 5);
    }
}
