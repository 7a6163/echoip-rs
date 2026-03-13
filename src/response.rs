use std::net::IpAddr;

use num_bigint::BigUint;
use serde::Serialize;
use serde::ser::Serializer;

use crate::user_agent::UserAgent;

fn serialize_ip<S: Serializer>(ip: &IpAddr, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(&ip.to_string())
}

fn serialize_decimal<S: Serializer>(val: &BigUint, s: S) -> Result<S::Ok, S::Error> {
    // Serialize as number if it fits in u64 (IPv4), otherwise as string (IPv6)
    let num_str = val.to_string();
    match num_str.parse::<u64>() {
        Ok(n) => s.serialize_u64(n),
        Err(_) => s.serialize_str(&num_str),
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_response(ip: IpAddr) -> Response {
        Response {
            ip,
            ip_decimal: crate::ip_util::to_decimal(ip),
            country: String::new(),
            country_iso: String::new(),
            country_eu: false,
            region_name: String::new(),
            region_code: String::new(),
            metro_code: 0,
            zip_code: String::new(),
            city: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            time_zone: String::new(),
            asn: String::new(),
            asn_org: String::new(),
            hostname: String::new(),
            user_agent: None,
        }
    }

    #[test]
    fn test_serialize_ipv4() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let resp = test_response(ip);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ip"], "127.0.0.1");
        assert_eq!(json["ip_decimal"], 2130706433u64);
    }

    #[test]
    fn test_serialize_ipv6() {
        let ip: IpAddr = "::1".parse().unwrap();
        let resp = test_response(ip);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ip"], "::1");
        assert_eq!(json["ip_decimal"], 1);
    }

    #[test]
    fn test_serialize_ipv6_large_decimal() {
        let ip: IpAddr = "8000::".parse().unwrap();
        let resp = test_response(ip);
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ip"], "8000::");
        // Large IPv6 decimal should be serialized as string
        assert_eq!(
            json["ip_decimal"],
            "170141183460469231731687303715884105728"
        );
    }

    #[test]
    fn test_skip_empty_fields() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let resp = test_response(ip);
        let json = serde_json::to_value(&resp).unwrap();
        // Empty strings should be skipped
        assert!(json.get("country").is_none());
        assert!(json.get("city").is_none());
        assert!(json.get("asn").is_none());
        // Zero values should be skipped
        assert!(json.get("latitude").is_none());
        assert!(json.get("metro_code").is_none());
        // False bool should be skipped
        assert!(json.get("country_eu").is_none());
        // None should be skipped
        assert!(json.get("user_agent").is_none());
    }

    #[test]
    fn test_include_non_empty_fields() {
        let ip: IpAddr = "127.0.0.1".parse().unwrap();
        let mut resp = test_response(ip);
        resp.country = "US".into();
        resp.latitude = 37.7;
        resp.country_eu = true;
        resp.metro_code = 807;
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["country"], "US");
        assert_eq!(json["latitude"], 37.7);
        assert_eq!(json["country_eu"], true);
        assert_eq!(json["metro_code"], 807);
    }

    #[test]
    fn test_port_response_serialize() {
        let resp = PortResponse {
            ip: "192.168.1.1".parse().unwrap(),
            port: 443,
            reachable: true,
        };
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ip"], "192.168.1.1");
        assert_eq!(json["port"], 443);
        assert_eq!(json["reachable"], true);
    }
}
