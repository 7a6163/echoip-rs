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

#[async_trait::async_trait]
pub trait GeoProvider: Send + Sync {
    async fn country(&self, ip: IpAddr) -> Option<Country>;
    async fn city(&self, ip: IpAddr) -> Option<City>;
    async fn asn(&self, ip: IpAddr) -> Option<Asn>;
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

#[async_trait::async_trait]
impl GeoProvider for SwappableGeoProvider {
    async fn country(&self, ip: IpAddr) -> Option<Country> {
        self.inner.load().country(ip).await
    }

    async fn city(&self, ip: IpAddr) -> Option<City> {
        self.inner.load().city(ip).await
    }

    async fn asn(&self, ip: IpAddr) -> Option<Asn> {
        self.inner.load().asn(ip).await
    }

    fn is_empty(&self) -> bool {
        self.inner.load().is_empty()
    }
}
