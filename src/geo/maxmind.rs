use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;

use maxminddb::Reader;
use serde::Deserialize;

use super::{Asn, City, Country, GeoProvider};

#[derive(Deserialize, Debug)]
struct CountryRecord {
    country: Option<CountryInfo>,
    registered_country: Option<CountryInfo>,
}

#[derive(Deserialize, Debug)]
struct CountryInfo {
    names: Option<std::collections::BTreeMap<String, String>>,
    iso_code: Option<String>,
    is_in_european_union: Option<bool>,
}

#[derive(Deserialize, Debug)]
struct CityRecord {
    city: Option<CityInfo>,
    subdivisions: Option<Vec<SubdivisionInfo>>,
    location: Option<LocationInfo>,
    postal: Option<PostalInfo>,
    country: Option<CountryIsoOnly>,
}

#[derive(Deserialize, Debug)]
struct CityInfo {
    names: Option<std::collections::BTreeMap<String, String>>,
}

#[derive(Deserialize, Debug)]
struct SubdivisionInfo {
    names: Option<std::collections::BTreeMap<String, String>>,
    iso_code: Option<String>,
}

#[derive(Deserialize, Debug)]
struct LocationInfo {
    latitude: Option<f64>,
    longitude: Option<f64>,
    metro_code: Option<u32>,
    time_zone: Option<String>,
}

#[derive(Deserialize, Debug)]
struct PostalInfo {
    code: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CountryIsoOnly {
    iso_code: Option<String>,
}

#[derive(Deserialize, Debug)]
struct AsnRecord {
    autonomous_system_number: Option<u32>,
    autonomous_system_organization: Option<String>,
}

pub struct MaxmindProvider {
    country_db: Option<Arc<Reader<Vec<u8>>>>,
    city_db: Option<Arc<Reader<Vec<u8>>>>,
    asn_db: Option<Arc<Reader<Vec<u8>>>>,
}

impl MaxmindProvider {
    pub fn open(
        country_path: Option<&str>,
        city_path: Option<&str>,
        asn_path: Option<&str>,
    ) -> Result<Self, maxminddb::MaxMindDbError> {
        let country_db = country_path
            .filter(|p| !p.is_empty() && Path::new(p).exists())
            .map(Reader::open_readfile)
            .transpose()?
            .map(Arc::new);

        let city_db = city_path
            .filter(|p| !p.is_empty() && Path::new(p).exists())
            .map(Reader::open_readfile)
            .transpose()?
            .map(Arc::new);

        let asn_db = asn_path
            .filter(|p| !p.is_empty() && Path::new(p).exists())
            .map(Reader::open_readfile)
            .transpose()?
            .map(Arc::new);

        Ok(Self {
            country_db,
            city_db,
            asn_db,
        })
    }
}

impl GeoProvider for MaxmindProvider {
    fn country(&self, ip: IpAddr) -> Option<Country> {
        let db = self.country_db.as_ref()?;
        let result = db.lookup(ip).ok()?;
        let record: CountryRecord = result.decode().ok()??;

        let mut name = String::new();
        let mut iso = String::new();
        let mut is_eu = false;

        if let Some(ref c) = record.country {
            if let Some(ref names) = c.names {
                if let Some(n) = names.get("en") {
                    name = n.to_string();
                }
            }
            if let Some(ref code) = c.iso_code {
                iso = code.clone();
            }
            is_eu = c.is_in_european_union.unwrap_or(false);
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

        Some(Country { name, iso, is_eu })
    }

    fn city(&self, ip: IpAddr) -> Option<City> {
        let db = self.city_db.as_ref()?;
        let result = db.lookup(ip).ok()?;
        let record: CityRecord = result.decode().ok()??;

        let mut city = City::default();

        if let Some(ref c) = record.city {
            if let Some(ref names) = c.names {
                if let Some(n) = names.get("en") {
                    city.name = n.to_string();
                }
            }
        }

        if let Some(ref subs) = record.subdivisions {
            if let Some(first) = subs.first() {
                if let Some(ref names) = first.names {
                    if let Some(n) = names.get("en") {
                        city.region_name = n.to_string();
                    }
                }
                if let Some(ref code) = first.iso_code {
                    city.region_code = code.clone();
                }
            }
        }

        if let Some(ref loc) = record.location {
            if let Some(lat) = loc.latitude {
                if !lat.is_nan() {
                    city.latitude = lat;
                }
            }
            if let Some(lon) = loc.longitude {
                if !lon.is_nan() {
                    city.longitude = lon;
                }
            }
            // Metro code is US only
            if let Some(metro) = loc.metro_code {
                let is_us =
                    record.country.as_ref().and_then(|c| c.iso_code.as_deref()) == Some("US");
                if metro > 0 && is_us {
                    city.metro_code = metro;
                }
            }
            if let Some(ref tz) = loc.time_zone {
                city.timezone = tz.clone();
            }
        }

        if let Some(ref postal) = record.postal {
            if let Some(ref code) = postal.code {
                city.postal_code = code.clone();
            }
        }

        Some(city)
    }

    fn asn(&self, ip: IpAddr) -> Option<Asn> {
        let db = self.asn_db.as_ref()?;
        let result = db.lookup(ip).ok()?;
        let record: AsnRecord = result.decode().ok()??;

        Some(Asn {
            number: record.autonomous_system_number.unwrap_or(0),
            organization: record.autonomous_system_organization.unwrap_or_default(),
        })
    }

    fn is_empty(&self) -> bool {
        self.country_db.is_none() && self.city_db.is_none() && self.asn_db.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures_dir() -> String {
        format!("{}/tests/fixtures", env!("CARGO_MANIFEST_DIR"))
    }

    fn country_db() -> String {
        format!("{}/GeoIP2-Country-Test.mmdb", fixtures_dir())
    }

    fn city_db() -> String {
        format!("{}/GeoIP2-City-Test.mmdb", fixtures_dir())
    }

    fn asn_db() -> String {
        format!("{}/GeoLite2-ASN-Test.mmdb", fixtures_dir())
    }

    #[test]
    fn test_open_all_none() {
        let provider = MaxmindProvider::open(None, None, None).unwrap();
        assert!(provider.is_empty());
    }

    #[test]
    fn test_open_empty_paths() {
        let provider = MaxmindProvider::open(Some(""), Some(""), Some("")).unwrap();
        assert!(provider.is_empty());
    }

    #[test]
    fn test_open_nonexistent_path() {
        let provider = MaxmindProvider::open(Some("/nonexistent.mmdb"), None, None).unwrap();
        assert!(provider.is_empty());
    }

    #[test]
    fn test_open_with_fixtures() {
        let provider =
            MaxmindProvider::open(Some(&country_db()), Some(&city_db()), Some(&asn_db())).unwrap();
        assert!(!provider.is_empty());
    }

    #[test]
    fn test_country_lookup() {
        let provider = MaxmindProvider::open(Some(&country_db()), None, None).unwrap();
        // 81.2.69.142 is a known test IP in MaxMind test data (GB)
        let ip: IpAddr = "81.2.69.142".parse().unwrap();
        let country = provider.country(ip);
        assert!(country.is_some());
        let c = country.unwrap();
        assert_eq!(c.iso, "GB");
        assert!(!c.name.is_empty());
    }

    #[test]
    fn test_country_eu() {
        let provider = MaxmindProvider::open(Some(&country_db()), None, None).unwrap();
        // 2.125.160.216 is a known test IP (GB, EU=true in some test data)
        // 89.160.20.112 is Sweden (SE, EU member)
        let ip: IpAddr = "89.160.20.112".parse().unwrap();
        let country = provider.country(ip);
        assert!(country.is_some());
        let c = country.unwrap();
        assert_eq!(c.iso, "SE");
        assert!(c.is_eu);
    }

    #[test]
    fn test_country_unknown_ip() {
        let provider = MaxmindProvider::open(Some(&country_db()), None, None).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        // localhost won't be in the test database
        let country = provider.country(ip);
        assert!(country.is_none());
    }

    #[test]
    fn test_city_lookup() {
        let provider = MaxmindProvider::open(None, Some(&city_db()), None).unwrap();
        let ip: IpAddr = "81.2.69.142".parse().unwrap();
        let city = provider.city(ip);
        assert!(city.is_some());
        let c = city.unwrap();
        assert!(!c.name.is_empty());
    }

    #[test]
    fn test_city_with_location() {
        let provider = MaxmindProvider::open(None, Some(&city_db()), None).unwrap();
        // 216.160.83.56 is a known test IP with location data
        let ip: IpAddr = "216.160.83.56".parse().unwrap();
        let city = provider.city(ip);
        assert!(city.is_some());
        let c = city.unwrap();
        assert!(c.latitude != 0.0 || c.longitude != 0.0);
    }

    #[test]
    fn test_city_unknown_ip() {
        let provider = MaxmindProvider::open(None, Some(&city_db()), None).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(provider.city(ip).is_none());
    }

    #[test]
    fn test_asn_lookup() {
        let provider = MaxmindProvider::open(None, None, Some(&asn_db())).unwrap();
        // 1.128.0.0 is a known test IP for ASN data
        let ip: IpAddr = "1.128.0.0".parse().unwrap();
        let asn = provider.asn(ip);
        assert!(asn.is_some());
        let a = asn.unwrap();
        assert!(a.number > 0);
        assert!(!a.organization.is_empty());
    }

    #[test]
    fn test_asn_unknown_ip() {
        let provider = MaxmindProvider::open(None, None, Some(&asn_db())).unwrap();
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        assert!(provider.asn(ip).is_none());
    }

    #[test]
    fn test_is_empty_partial() {
        let provider = MaxmindProvider::open(Some(&country_db()), None, None).unwrap();
        assert!(!provider.is_empty());
    }
}
