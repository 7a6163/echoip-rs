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
