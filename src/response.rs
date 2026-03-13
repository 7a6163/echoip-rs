use std::net::IpAddr;

use num_bigint::BigUint;
use serde::ser::Serializer;
use serde::Serialize;

use crate::user_agent::UserAgent;

fn serialize_ip<S: Serializer>(ip: &IpAddr, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&ip.to_string())
}

fn serialize_decimal<S: Serializer>(val: &BigUint, s: S) -> Result<S::Ok, S::Error> {
    use serde::ser::Error;
    // Serialize as number if it fits in u64, otherwise as string
    let num_str = val.to_string();
    let n: u64 = num_str.parse().map_err(Error::custom)?;
    s.serialize_u64(n)
}

fn is_false(v: &bool) -> bool {
    !v
}

fn is_zero_u32(v: &u32) -> bool {
    *v == 0
}

fn is_zero_f64(v: &f64) -> bool {
    *v == 0.0
}

#[derive(Debug, Clone, Serialize)]
pub struct Response {
    #[serde(serialize_with = "serialize_ip")]
    pub ip: IpAddr,
    #[serde(serialize_with = "serialize_decimal")]
    pub ip_decimal: BigUint,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub country: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub country_iso: String,
    #[serde(skip_serializing_if = "is_false")]
    pub country_eu: bool,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub region_name: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub region_code: String,
    #[serde(skip_serializing_if = "is_zero_u32")]
    pub metro_code: u32,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub zip_code: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub city: String,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub latitude: f64,
    #[serde(skip_serializing_if = "is_zero_f64")]
    pub longitude: f64,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub time_zone: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub asn: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub asn_org: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub hostname: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<UserAgent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortResponse {
    #[serde(serialize_with = "serialize_ip")]
    pub ip: IpAddr,
    pub port: u16,
    pub reachable: bool,
}
