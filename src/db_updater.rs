use std::io::Read as _;
use std::path::{Path, PathBuf};

use flate2::read::GzDecoder;
use tracing::{error, info};

#[derive(Debug, thiserror::Error)]
pub enum DbUpdateError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("No .mmdb file found in archive for {0}")]
    MmdbNotFound(String),
    #[error("Downloaded file is not a valid MMDB: {0}")]
    Validation(String),
}

/// MaxMind authentication method.
#[derive(Debug, Clone)]
pub enum MaxmindAuth {
    /// Legacy API: license key as query parameter
    /// Env: `GEOIP_LICENSE_KEY`
    LegacyKey(String),
    /// New API: HTTP Basic Auth with account ID + license key
    /// Env: `MAXMIND_ACCOUNT_ID` + `MAXMIND_LICENSE_KEY`
    BasicAuth { account_id: String, license_key: String },
}

impl MaxmindAuth {
    /// Detect authentication from environment variables.
    /// Prefers new API (MAXMIND_ACCOUNT_ID + MAXMIND_LICENSE_KEY) over legacy (GEOIP_LICENSE_KEY).
    pub fn from_env() -> Option<Self> {
        let account_id = std::env::var("MAXMIND_ACCOUNT_ID").ok().filter(|s| !s.is_empty());
        let new_key = std::env::var("MAXMIND_LICENSE_KEY").ok().filter(|s| !s.is_empty());
        let legacy_key = std::env::var("GEOIP_LICENSE_KEY").ok().filter(|s| !s.is_empty());

        if let (Some(id), Some(key)) = (account_id, new_key) {
            Some(MaxmindAuth::BasicAuth {
                account_id: id,
                license_key: key,
            })
        } else {
            legacy_key.map(MaxmindAuth::LegacyKey)
        }
    }
}

#[derive(Debug, Default)]
pub struct DownloadResult {
    pub country_path: Option<PathBuf>,
    pub city_path: Option<PathBuf>,
    pub asn_path: Option<PathBuf>,
    pub ip66_path: Option<PathBuf>,
}

pub struct DbUpdater {
    data_dir: PathBuf,
    auth: Option<MaxmindAuth>,
    client: reqwest::Client,
}

impl DbUpdater {
    pub fn new(data_dir: PathBuf, auth: Option<MaxmindAuth>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");

        Self {
            data_dir,
            auth,
            client,
        }
    }

    pub async fn download_all(&self) -> DownloadResult {
        std::fs::create_dir_all(&self.data_dir).ok();

        let mut result = DownloadResult::default();

        // MaxMind databases (requires authentication)
        if self.auth.is_some() {
            match self.download_maxmind("GeoLite2-Country").await {
                Ok(path) => result.country_path = Some(path),
                Err(e) => error!("Failed to download GeoLite2-Country: {e}"),
            }
            match self.download_maxmind("GeoLite2-City").await {
                Ok(path) => result.city_path = Some(path),
                Err(e) => error!("Failed to download GeoLite2-City: {e}"),
            }
            match self.download_maxmind("GeoLite2-ASN").await {
                Ok(path) => result.asn_path = Some(path),
                Err(e) => error!("Failed to download GeoLite2-ASN: {e}"),
            }
        }

        // ip66.dev database (no key needed)
        match self.download_ip66().await {
            Ok(path) => result.ip66_path = Some(path),
            Err(e) => error!("Failed to download ip66.mmdb: {e}"),
        }

        result
    }

    async fn download_maxmind(
        &self,
        edition_id: &str,
    ) -> Result<PathBuf, DbUpdateError> {
        let auth = self.auth.as_ref().ok_or_else(|| {
            DbUpdateError::Validation("No MaxMind credentials configured".to_string())
        })?;

        info!("Downloading {edition_id}...");

        let req = match auth {
            MaxmindAuth::BasicAuth { account_id, license_key } => {
                // New API: Basic Auth + new endpoint
                let url = format!(
                    "https://download.maxmind.com/geoip/databases/{edition_id}/download?suffix=tar.gz"
                );
                self.client.get(&url).basic_auth(account_id, Some(license_key))
            }
            MaxmindAuth::LegacyKey(key) => {
                // Legacy API: license key in query parameter
                let url = format!(
                    "https://download.maxmind.com/app/geoip_download?edition_id={edition_id}&license_key={key}&suffix=tar.gz"
                );
                self.client.get(&url)
            }
        };

        let resp = req.send().await.map_err(|e| {
            // Strip URL to avoid leaking license key in logs
            DbUpdateError::Validation(format!(
                "HTTP request failed for {edition_id}: {}",
                e.without_url()
            ))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(DbUpdateError::Validation(format!(
                "MaxMind returned {status} for {edition_id}: {body}"
            )));
        }

        let bytes = resp.bytes().await?;
        self.extract_mmdb(edition_id, &bytes)
    }

    fn extract_mmdb(
        &self,
        edition_id: &str,
        tar_gz_bytes: &[u8],
    ) -> Result<PathBuf, DbUpdateError> {
        let decoder = GzDecoder::new(tar_gz_bytes);
        let mut archive = tar::Archive::new(decoder);

        let target_name = format!("{edition_id}.mmdb");
        let final_path = self.data_dir.join(&target_name);
        let tmp_path = self.data_dir.join(format!("{target_name}.tmp"));

        let mut found = false;
        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n == target_name)
            {
                let mut buf = Vec::new();
                entry.read_to_end(&mut buf)?;
                std::fs::write(&tmp_path, &buf)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(DbUpdateError::MmdbNotFound(edition_id.to_string()));
        }

        validate_mmdb(&tmp_path)?;
        std::fs::rename(&tmp_path, &final_path)?;
        info!("Downloaded {edition_id} -> {}", final_path.display());

        Ok(final_path)
    }

    async fn download_ip66(&self) -> Result<PathBuf, DbUpdateError> {
        let url = "https://downloads.ip66.dev/db/ip66.mmdb";

        info!("Downloading ip66.mmdb...");
        let resp = self.client.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            return Err(DbUpdateError::Validation(format!(
                "ip66.dev returned {status}"
            )));
        }

        let bytes = resp.bytes().await?;

        let final_path = self.data_dir.join("ip66.mmdb");
        let tmp_path = self.data_dir.join("ip66.mmdb.tmp");

        std::fs::write(&tmp_path, &bytes)?;
        validate_mmdb(&tmp_path)?;
        std::fs::rename(&tmp_path, &final_path)?;
        info!("Downloaded ip66.mmdb -> {}", final_path.display());

        Ok(final_path)
    }
}

fn validate_mmdb(path: &Path) -> Result<(), DbUpdateError> {
    maxminddb::Reader::open_readfile(path).map_err(|e| {
        std::fs::remove_file(path).ok();
        DbUpdateError::Validation(format!("{}: {e}", path.display()))
    })?;
    Ok(())
}

/// Build a geo provider from available database paths.
pub fn build_provider(
    country_path: Option<&str>,
    city_path: Option<&str>,
    asn_path: Option<&str>,
    ip66_path: Option<&str>,
) -> Box<dyn crate::geo::GeoProvider> {
    use crate::geo::composite::CompositeProvider;
    use crate::geo::ip66::Ip66Provider;
    use crate::geo::maxmind::MaxmindProvider;
    use crate::geo::GeoProvider;

    let mut providers: Vec<Box<dyn GeoProvider>> = Vec::new();

    let has_maxmind = [country_path, city_path, asn_path]
        .iter()
        .any(|p| p.is_some_and(|s| !s.is_empty()));

    if has_maxmind {
        let c = country_path.filter(|s| !s.is_empty());
        let ci = city_path.filter(|s| !s.is_empty());
        let a = asn_path.filter(|s| !s.is_empty());
        match MaxmindProvider::open(c, ci, a) {
            Ok(p) => {
                info!("Loaded MaxMind GeoIP databases");
                providers.push(Box::new(p));
            }
            Err(e) => error!("Failed to open MaxMind databases: {e}"),
        }
    }

    if let Some(path) = ip66_path.filter(|s| !s.is_empty()) {
        match Ip66Provider::open(path) {
            Ok(p) => {
                info!("Loaded ip66.dev database");
                providers.push(Box::new(p));
            }
            Err(e) => error!("Failed to open ip66 database: {e}"),
        }
    }

    if providers.is_empty() {
        Box::new(MaxmindProvider::open(None, None, None).unwrap())
    } else if providers.len() == 1 {
        providers.into_iter().next().unwrap()
    } else {
        Box::new(CompositeProvider::new(providers))
    }
}

/// Resolve effective DB paths: CLI flags take priority over auto-downloaded paths.
pub fn resolve_paths(
    cli_path: &str,
    downloaded: &Option<PathBuf>,
) -> Option<String> {
    if !cli_path.is_empty() {
        return Some(cli_path.to_string());
    }
    downloaded
        .as_ref()
        .map(|p| p.to_string_lossy().to_string())
}
