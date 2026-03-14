pub mod composite;
pub mod ip66;
pub mod maxmind;

use std::net::IpAddr;
use std::sync::Arc;

use arc_swap::ArcSwap;

#[derive(Debug, Clone, Default)]
pub struct Country {
    pub name: String,
    pub iso: String,
    pub is_eu: bool,
}

#[derive(Debug, Clone, Default)]
pub struct City {
    pub name: String,
    pub latitude: f64,
    pub longitude: f64,
    pub postal_code: String,
    pub timezone: String,
    pub metro_code: u32,
    pub region_name: String,
    pub region_code: String,
}

#[derive(Debug, Clone, Default)]
pub struct Asn {
    pub number: u32,
    pub organization: String,
}

pub trait GeoProvider: Send + Sync {
    fn country(&self, ip: IpAddr) -> Option<Country>;
    fn city(&self, ip: IpAddr) -> Option<City>;
    fn asn(&self, ip: IpAddr) -> Option<Asn>;
    fn is_empty(&self) -> bool;
}

/// A geo provider that can be hot-swapped at runtime without locking.
pub struct SwappableGeoProvider {
    inner: ArcSwap<Box<dyn GeoProvider>>,
}

impl SwappableGeoProvider {
    pub fn new(provider: Box<dyn GeoProvider>) -> Self {
        Self {
            inner: ArcSwap::from_pointee(provider),
        }
    }

    pub fn swap(&self, new_provider: Box<dyn GeoProvider>) {
        self.inner.store(Arc::new(new_provider));
    }
}

impl GeoProvider for SwappableGeoProvider {
    fn country(&self, ip: IpAddr) -> Option<Country> {
        self.inner.load().country(ip)
    }

    fn city(&self, ip: IpAddr) -> Option<City> {
        self.inner.load().city(ip)
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
        self.inner.load().asn(ip)
    }

    fn is_empty(&self) -> bool {
        self.inner.load().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProvider {
        name: String,
    }

    impl GeoProvider for TestProvider {
        fn country(&self, _ip: IpAddr) -> Option<Country> {
            Some(Country {
                name: self.name.clone(),
                iso: "XX".into(),
                is_eu: false,
            })
        }
        fn city(&self, _ip: IpAddr) -> Option<City> {
            Some(City {
                name: self.name.clone(),
                ..Default::default()
            })
        }
        fn asn(&self, _ip: IpAddr) -> Option<Asn> {
            Some(Asn {
                number: 1,
                organization: self.name.clone(),
            })
        }
        fn is_empty(&self) -> bool {
            false
        }
    }

    struct EmptyProvider;

    impl GeoProvider for EmptyProvider {
        fn country(&self, _ip: IpAddr) -> Option<Country> {
            None
        }
        fn city(&self, _ip: IpAddr) -> Option<City> {
            None
        }
        fn asn(&self, _ip: IpAddr) -> Option<Asn> {
            None
        }
        fn is_empty(&self) -> bool {
            true
        }
    }

    fn ip() -> IpAddr {
        "127.0.0.1".parse().unwrap()
    }

    #[test]
    fn test_swappable_provider_delegates() {
        let provider = SwappableGeoProvider::new(Box::new(TestProvider {
            name: "first".into(),
        }));
        assert_eq!(provider.country(ip()).unwrap().name, "first");
        assert_eq!(provider.city(ip()).unwrap().name, "first");
        assert_eq!(provider.asn(ip()).unwrap().organization, "first");
        assert!(!provider.is_empty());
    }

    #[test]
    fn test_swappable_provider_swap() {
        let provider = SwappableGeoProvider::new(Box::new(TestProvider {
            name: "first".into(),
        }));
        assert_eq!(provider.country(ip()).unwrap().name, "first");

        provider.swap(Box::new(TestProvider {
            name: "second".into(),
        }));
        assert_eq!(provider.country(ip()).unwrap().name, "second");
        assert_eq!(provider.city(ip()).unwrap().name, "second");
        assert_eq!(provider.asn(ip()).unwrap().organization, "second");
    }

    #[test]
    fn test_swappable_provider_empty() {
        let provider = SwappableGeoProvider::new(Box::new(EmptyProvider));
        assert!(provider.is_empty());
        assert!(provider.country(ip()).is_none());
        assert!(provider.city(ip()).is_none());
        assert!(provider.asn(ip()).is_none());
    }

    #[test]
    fn test_default_structs() {
        let country = Country::default();
        assert!(country.name.is_empty());
        assert!(country.iso.is_empty());
        assert!(!country.is_eu);

        let city = City::default();
        assert!(city.name.is_empty());
        assert_eq!(city.latitude, 0.0);
        assert_eq!(city.longitude, 0.0);

        let asn = Asn::default();
        assert_eq!(asn.number, 0);
        assert!(asn.organization.is_empty());
    }
}
