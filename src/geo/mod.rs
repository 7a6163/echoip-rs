pub mod composite;
pub mod ip66;
pub mod maxmind;

use std::net::IpAddr;

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
