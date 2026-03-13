use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "echoip", about = "IP address lookup service")]
pub struct Config {
    /// Path to GeoIP country database
    #[arg(short = 'f', long = "country-db", default_value = "")]
    pub country_db: String,

    /// Path to GeoIP city database
    #[arg(short = 'c', long = "city-db", default_value = "")]
    pub city_db: String,

    /// Path to GeoIP ASN database
    #[arg(short = 'a', long = "asn-db", default_value = "")]
    pub asn_db: String,

    /// Listening address
    #[arg(short = 'l', long = "listen", default_value = ":8080")]
    pub listen: String,

    /// Perform reverse hostname lookups
    #[arg(short = 'r', long = "reverse-lookup")]
    pub reverse_lookup: bool,

    /// Enable port lookup
    #[arg(short = 'p', long = "port-lookup")]
    pub port_lookup: bool,

    /// Path to template directory
    #[arg(short = 't', long = "template", default_value = "html")]
    pub template: String,

    /// Size of response cache (0 to disable)
    #[arg(short = 'C', long = "cache-size", default_value_t = 0)]
    pub cache_size: usize,

    /// Enable profiling/debug handlers
    #[arg(short = 'P', long = "profile")]
    pub profile: bool,

    /// Show sponsor logo
    #[arg(short = 's', long = "sponsor")]
    pub sponsor: bool,

    /// Headers to trust for remote IP (repeatable)
    #[arg(short = 'H', long = "trusted-header")]
    pub trusted_headers: Vec<String>,

    /// Path to ip66.dev MMDB database
    #[arg(long = "ip66-db")]
    pub ip66_db: Option<String>,

    /// Directory for auto-downloaded databases
    #[arg(short = 'd', long = "data-dir", default_value = "data")]
    pub data_dir: String,

    /// Auto-update interval in hours (0 to disable periodic updates)
    #[arg(long = "update-interval", default_value_t = 0)]
    pub update_interval: u64,

    /// Disable automatic database download on startup
    #[arg(long = "no-auto-download")]
    pub no_auto_download: bool,
}

impl Config {
    pub fn listen_addr(&self) -> String {
        let listen = &self.listen;
        if listen.starts_with(':') {
            format!("0.0.0.0{listen}")
        } else {
            listen.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_listen_addr_with_colon() {
        let config = Config {
            listen: ":8080".into(),
            country_db: String::new(),
            city_db: String::new(),
            asn_db: String::new(),
            reverse_lookup: false,
            port_lookup: false,
            template: String::new(),
            cache_size: 0,
            profile: false,
            sponsor: false,
            trusted_headers: vec![],
            ip66_db: None,
            data_dir: "data".into(),
            update_interval: 0,
            no_auto_download: true,
        };
        assert_eq!(config.listen_addr(), "0.0.0.0:8080");
    }

    #[test]
    fn test_listen_addr_full() {
        let config = Config {
            listen: "127.0.0.1:3000".into(),
            country_db: String::new(),
            city_db: String::new(),
            asn_db: String::new(),
            reverse_lookup: false,
            port_lookup: false,
            template: String::new(),
            cache_size: 0,
            profile: false,
            sponsor: false,
            trusted_headers: vec![],
            ip66_db: None,
            data_dir: "data".into(),
            update_interval: 0,
            no_auto_download: true,
        };
        assert_eq!(config.listen_addr(), "127.0.0.1:3000");
    }
}
