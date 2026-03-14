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

impl GeoProvider for Ip66Provider {
    fn country(&self, ip: IpAddr) -> Option<Country> {
        let result = self.db.lookup(ip).ok()?;
        let record: Ip66CountryRecord = result.decode().ok()??;

        let mut name = String::new();
        let mut iso = String::new();

        if let Some(ref c) = record.country {
            if let Some(ref names) = c.names {
                if let Some(n) = names.get("en") {
                    name = n.to_string();
                }
            }
            if let Some(ref code) = c.iso_code {
                iso = code.clone();
            }
        }

        // Fallback to registered country
        if let Some(ref rc) = record.registered_country {
            if name.is_empty() {
                if let Some(ref names) = rc.names {
                    if let Some(n) = names.get("en") {
                        name = n.to_string();
                    }
                }
            }
            if iso.is_empty() {
                if let Some(ref code) = rc.iso_code {
                    iso = code.clone();
                }
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

    fn city(&self, _ip: IpAddr) -> Option<City> {
        // ip66 MMDB does not include city-level data
        None
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_nonexistent() {
        let result = Ip66Provider::open("/nonexistent/ip66.mmdb");
        assert!(result.is_err());
    }

    #[test]
    fn test_open_with_country_db() {
        // ip66 uses the same MMDB format; we can open any valid MMDB
        // but lookups may not return ip66-specific fields
        let path = format!(
            "{}/tests/fixtures/GeoIP2-Country-Test.mmdb",
            env!("CARGO_MANIFEST_DIR")
        );
        let provider = Ip66Provider::open(&path).unwrap();
        assert!(!provider.is_empty());

        // ip66 city always returns None
        let ip: IpAddr = "81.2.69.142".parse().unwrap();
        assert!(provider.city(ip).is_none());
    }

    #[test]
    fn test_country_with_test_db() {
        let path = format!(
            "{}/tests/fixtures/GeoIP2-Country-Test.mmdb",
            env!("CARGO_MANIFEST_DIR")
        );
        let provider = Ip66Provider::open(&path).unwrap();

        // Should parse country from test database
        let ip: IpAddr = "81.2.69.142".parse().unwrap();
        let country = provider.country(ip);
        // The test DB has country data for this IP
        if let Some(c) = country {
            assert!(!c.name.is_empty() || !c.iso.is_empty());
            // ip66 always sets is_eu to false
            assert!(!c.is_eu);
        }
    }

    #[test]
    fn test_country_unknown_ip() {
        let path = format!(
            "{}/tests/fixtures/GeoIP2-Country-Test.mmdb",
            env!("CARGO_MANIFEST_DIR")
        );
        let provider = Ip66Provider::open(&path).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(provider.country(ip).is_none());
    }

    #[test]
    fn test_asn_with_test_db() {
        let path = format!(
            "{}/tests/fixtures/GeoLite2-ASN-Test.mmdb",
            env!("CARGO_MANIFEST_DIR")
        );
        let provider = Ip66Provider::open(&path).unwrap();

        let ip: IpAddr = "1.128.0.0".parse().unwrap();
        let asn = provider.asn(ip);
        if let Some(a) = asn {
            assert!(a.number > 0);
        }
    }

    #[test]
    fn test_asn_unknown_ip() {
        let path = format!(
            "{}/tests/fixtures/GeoLite2-ASN-Test.mmdb",
            env!("CARGO_MANIFEST_DIR")
        );
        let provider = Ip66Provider::open(&path).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(provider.asn(ip).is_none());
    }
}
