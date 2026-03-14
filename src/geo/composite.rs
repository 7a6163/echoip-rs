use std::net::IpAddr;

use super::{Asn, City, Country, GeoProvider};

pub struct CompositeProvider {
    providers: Vec<Box<dyn GeoProvider>>,
}

impl CompositeProvider {
    pub fn new(providers: Vec<Box<dyn GeoProvider>>) -> Self {
        Self { providers }
    }
}

impl GeoProvider for CompositeProvider {
    fn country(&self, ip: IpAddr) -> Option<Country> {
        for provider in &self.providers {
            if let Some(country) = provider.country(ip) {
                if !country.name.is_empty() || !country.iso.is_empty() {
                    return Some(country);
                }
            }
        }
        None
    }

    fn city(&self, ip: IpAddr) -> Option<City> {
        for provider in &self.providers {
            if let Some(city) = provider.city(ip) {
                if !city.name.is_empty() || city.latitude != 0.0 || city.longitude != 0.0 {
                    return Some(city);
                }
            }
        }
        None
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
        for provider in &self.providers {
            if let Some(asn) = provider.asn(ip) {
                if asn.number > 0 {
                    return Some(asn);
                }
            }
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.providers.iter().all(|p| p.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        country: Option<Country>,
        city: Option<City>,
        asn: Option<Asn>,
        empty: bool,
    }

    impl GeoProvider for MockProvider {
        fn country(&self, _ip: IpAddr) -> Option<Country> {
            self.country.clone()
        }
        fn city(&self, _ip: IpAddr) -> Option<City> {
            self.city.clone()
        }
        fn asn(&self, _ip: IpAddr) -> Option<Asn> {
            self.asn.clone()
        }
        fn is_empty(&self) -> bool {
            self.empty
        }
    }

    fn ip() -> IpAddr {
        "127.0.0.1".parse().unwrap()
    }

    fn empty_provider() -> Box<dyn GeoProvider> {
        Box::new(MockProvider {
            country: None,
            city: None,
            asn: None,
            empty: true,
        })
    }

    fn full_provider(name: &str) -> Box<dyn GeoProvider> {
        Box::new(MockProvider {
            country: Some(Country {
                name: name.to_string(),
                iso: "XX".to_string(),
                is_eu: false,
            }),
            city: Some(City {
                name: name.to_string(),
                latitude: 1.0,
                longitude: 2.0,
                ..Default::default()
            }),
            asn: Some(Asn {
                number: 12345,
                organization: name.to_string(),
            }),
            empty: false,
        })
    }

    #[test]
    fn test_composite_uses_first_valid_provider() {
        let composite =
            CompositeProvider::new(vec![full_provider("Primary"), full_provider("Fallback")]);
        assert_eq!(composite.country(ip()).unwrap().name, "Primary");
        assert_eq!(composite.city(ip()).unwrap().name, "Primary");
        assert_eq!(composite.asn(ip()).unwrap().organization, "Primary");
    }

    #[test]
    fn test_composite_falls_back_to_second() {
        let composite = CompositeProvider::new(vec![empty_provider(), full_provider("Fallback")]);
        assert_eq!(composite.country(ip()).unwrap().name, "Fallback");
        assert_eq!(composite.city(ip()).unwrap().name, "Fallback");
        assert_eq!(composite.asn(ip()).unwrap().organization, "Fallback");
    }

    #[test]
    fn test_composite_all_empty() {
        let composite = CompositeProvider::new(vec![empty_provider(), empty_provider()]);
        assert!(composite.country(ip()).is_none());
        assert!(composite.city(ip()).is_none());
        assert!(composite.asn(ip()).is_none());
    }

    #[test]
    fn test_composite_skips_empty_names() {
        // Provider with empty country name/iso should be skipped
        let partial = Box::new(MockProvider {
            country: Some(Country {
                name: String::new(),
                iso: String::new(),
                is_eu: false,
            }),
            city: Some(City::default()),
            asn: Some(Asn {
                number: 0,
                organization: String::new(),
            }),
            empty: false,
        });
        let composite = CompositeProvider::new(vec![partial, full_provider("Good")]);
        assert_eq!(composite.country(ip()).unwrap().name, "Good");
        assert_eq!(composite.city(ip()).unwrap().name, "Good");
        assert_eq!(composite.asn(ip()).unwrap().organization, "Good");
    }

    #[test]
    fn test_is_empty() {
        let all_empty = CompositeProvider::new(vec![empty_provider(), empty_provider()]);
        assert!(all_empty.is_empty());

        let has_one = CompositeProvider::new(vec![empty_provider(), full_provider("X")]);
        assert!(!has_one.is_empty());
    }

    #[test]
    fn test_no_providers() {
        let composite = CompositeProvider::new(vec![]);
        assert!(composite.country(ip()).is_none());
        assert!(composite.city(ip()).is_none());
        assert!(composite.asn(ip()).is_none());
        assert!(composite.is_empty());
    }
}
