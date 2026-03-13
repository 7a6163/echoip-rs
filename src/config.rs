use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(name = "echoip-rs", about = "IP address lookup service")]
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

    /// Enable ip66.dev as geo provider
    #[arg(long = "ip66")]
    pub ip66: bool,

    /// Custom ip66.dev API base URL
    #[arg(long = "ip66-url")]
    pub ip66_url: Option<String>,
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
