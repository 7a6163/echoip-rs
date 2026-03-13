use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use maxminddb::Reader;
use serde::Deserialize;

use super::{Asn, City, Country, GeoProvider};

#[derive(Deserialize, Debug)]
struct Ip66CountryRecord {
    country: Option<Ip66CountryInfo>,
    registered_country: Option<Ip66CountryInfo>,
}

#[derive(Deserialize, Debug)]
struct Ip66CountryInfo {
    names: Option<std::collections::BTreeMap<String, String>>,
    iso_code: Option<String>,
}

#[derive(Deserialize, Debug)]
struct Ip66AsnRecord {
    autonomous_system_number: Option<u32>,
    autonomous_system_organization: Option<String>,
}

pub struct Ip66Provider {
    db: Arc<Reader<Vec<u8>>>,
}

impl Ip66Provider {
    pub fn open(path: &str) -> Result<Self, maxminddb::MaxMindDbError> {
        if !Path::new(path).exists() {
            return Err(maxminddb::MaxMindDbError::invalid_database(format!(
                "ip66 database not found: {path}"
            )));
        }
        let db = Reader::open_readfile(path)?;
        Ok(Self { db: Arc::new(db) })
    }
}

#[async_trait::async_trait]
impl GeoProvider for Ip66Provider {
    async fn country(&self, ip: IpAddr) -> Option<Country> {
        let result = self.db.lookup(ip).ok()?;
        let record: Ip66CountryRecord = result.decode().ok()??;

        let mut name = String::new();
        let mut iso = String::new();

        if let Some(ref c) = record.country {
            if let Some(ref names) = c.names
                && let Some(n) = names.get("en")
            {
                name = n.to_string();
            }
            if let Some(ref code) = c.iso_code {
                iso = code.clone();
            }
        }

        // Fallback to registered country
        if let Some(ref rc) = record.registered_country {
            if name.is_empty()
                && let Some(ref names) = rc.names
                && let Some(n) = names.get("en")
            {
                name = n.to_string();
            }
            if iso.is_empty()
                && let Some(ref code) = rc.iso_code
            {
                iso = code.clone();
            }
        }

        if name.is_empty() && iso.is_empty() {
            return None;
        }

        Some(Country {
            name,
            iso,
            is_eu: false,
        })
    }

    async fn city(&self, _ip: IpAddr) -> Option<City> {
        // ip66 MMDB does not include city-level data
        None
    }

    async fn asn(&self, ip: IpAddr) -> Option<Asn> {
        let result = self.db.lookup(ip).ok()?;
        let record: Ip66AsnRecord = result.decode().ok()??;

        let number = record.autonomous_system_number.unwrap_or(0);
        if number == 0 {
            return None;
        }

        Some(Asn {
            number,
            organization: record.autonomous_system_organization.unwrap_or_default(),
        })
    }

    fn is_empty(&self) -> bool {
        false
    }
}
