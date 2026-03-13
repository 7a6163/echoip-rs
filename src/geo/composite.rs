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

#[async_trait::async_trait]
impl GeoProvider for CompositeProvider {
    async fn country(&self, ip: IpAddr) -> Option<Country> {
        for provider in &self.providers {
            if let Some(country) = provider.country(ip).await
                && (!country.name.is_empty() || !country.iso.is_empty()) {
                    return Some(country);
                }
        }
        None
    }

    async fn city(&self, ip: IpAddr) -> Option<City> {
        for provider in &self.providers {
            if let Some(city) = provider.city(ip).await
                && (!city.name.is_empty() || city.latitude != 0.0 || city.longitude != 0.0) {
                    return Some(city);
                }
        }
        None
    }

    async fn asn(&self, ip: IpAddr) -> Option<Asn> {
        for provider in &self.providers {
            if let Some(asn) = provider.asn(ip).await
                && asn.number > 0 {
                    return Some(asn);
                }
        }
        None
    }

    fn is_empty(&self) -> bool {
        self.providers.iter().all(|p| p.is_empty())
    }
}
