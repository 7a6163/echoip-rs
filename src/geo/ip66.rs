use std::net::IpAddr;
use std::sync::Arc;

use reqwest::Client;
use serde::Deserialize;
use tokio::sync::RwLock;

use super::{Asn, City, Country, GeoProvider};

#[derive(Debug, Deserialize, Clone, Default)]
struct Ip66Response {
    #[serde(default)]
    country: String,
    #[serde(default)]
    country_code: String,
    #[serde(default)]
    city: String,
    #[serde(default)]
    latitude: f64,
    #[serde(default)]
    longitude: f64,
    #[serde(default)]
    asn: u32,
    #[serde(default)]
    as_org: String,
    #[serde(default)]
    timezone: String,
    #[serde(default)]
    zip_code: String,
    #[serde(default)]
    region: String,
    #[serde(default)]
    region_code: String,
    #[serde(default)]
    is_eu: bool,
}

struct CachedLookup {
    ip: IpAddr,
    response: Ip66Response,
}

pub struct Ip66Provider {
    client: Client,
    base_url: String,
    cached: Arc<RwLock<Option<CachedLookup>>>,
}

impl Ip66Provider {
    pub fn new(base_url: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url: base_url.unwrap_or_else(|| "https://ip66.dev/api".to_string()),
            cached: Arc::new(RwLock::new(None)),
        }
    }

    async fn fetch(&self, ip: IpAddr) -> Option<Ip66Response> {
        // Check cache first
        {
            let cached = self.cached.read().await;
            if let Some(ref entry) = *cached
                && entry.ip == ip {
                    return Some(entry.response.clone());
                }
        }

        let url = format!("{}/{}", self.base_url, ip);
        let resp = self.client.get(&url).send().await.ok()?;
        let data: Ip66Response = resp.json().await.ok()?;

        // Cache result
        {
            let mut cached = self.cached.write().await;
            *cached = Some(CachedLookup {
                ip,
                response: data.clone(),
            });
        }

        Some(data)
    }
}

#[async_trait::async_trait]
impl GeoProvider for Ip66Provider {
    async fn country(&self, ip: IpAddr) -> Option<Country> {
        let data = self.fetch(ip).await?;
        if data.country.is_empty() && data.country_code.is_empty() {
            return None;
        }
        Some(Country {
            name: data.country,
            iso: data.country_code,
            is_eu: data.is_eu,
        })
    }

    async fn city(&self, ip: IpAddr) -> Option<City> {
        let data = self.fetch(ip).await?;
        if data.city.is_empty() && data.latitude == 0.0 && data.longitude == 0.0 {
            return None;
        }
        Some(City {
            name: data.city,
            latitude: data.latitude,
            longitude: data.longitude,
            postal_code: data.zip_code,
            timezone: data.timezone,
            metro_code: 0,
            region_name: data.region,
            region_code: data.region_code,
        })
    }

    async fn asn(&self, ip: IpAddr) -> Option<Asn> {
        let data = self.fetch(ip).await?;
        if data.asn == 0 {
            return None;
        }
        Some(Asn {
            number: data.asn,
            organization: data.as_org,
        })
    }

    fn is_empty(&self) -> bool {
        false
    }
}
