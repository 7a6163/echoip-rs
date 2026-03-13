use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing::info;

use echoip::cache::Cache;
use echoip::config::Config;
use echoip::db_updater::{self, DbUpdater};
use echoip::geo::SwappableGeoProvider;
use echoip::server::{build_router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();
    let license_key = std::env::var("GEOIP_LICENSE_KEY").ok().filter(|k| !k.is_empty());

    // Auto-download databases on startup
    let download_result = if !config.no_auto_download && license_key.is_some() {
        let updater = DbUpdater::new(config.data_dir.clone().into(), license_key.clone());
        info!("Auto-downloading GeoIP databases to {}", config.data_dir);
        Some(updater.download_all().await)
    } else if !config.no_auto_download {
        // No license key: only download ip66
        let updater = DbUpdater::new(config.data_dir.clone().into(), None);
        info!("Auto-downloading ip66 database to {}", config.data_dir);
        Some(updater.download_all().await)
    } else {
        None
    };

    // Resolve effective paths: CLI flags > auto-downloaded > none
    let country_path = db_updater::resolve_paths(
        &config.country_db,
        &download_result.as_ref().and_then(|r| r.country_path.clone()),
    );
    let city_path = db_updater::resolve_paths(
        &config.city_db,
        &download_result.as_ref().and_then(|r| r.city_path.clone()),
    );
    let asn_path = db_updater::resolve_paths(
        &config.asn_db,
        &download_result.as_ref().and_then(|r| r.asn_path.clone()),
    );
    let ip66_path = db_updater::resolve_paths(
        config.ip66_db.as_deref().unwrap_or(""),
        &download_result.as_ref().and_then(|r| r.ip66_path.clone()),
    );

    let provider = db_updater::build_provider(
        country_path.as_deref(),
        city_path.as_deref(),
        asn_path.as_deref(),
        ip66_path.as_deref(),
    );

    let geo = Arc::new(SwappableGeoProvider::new(provider));
    let cache = Arc::new(RwLock::new(Cache::new(config.cache_size)));

    // Load templates
    let tera = if Path::new(&config.template).exists() {
        let glob = format!("{}/*", config.template);
        match tera::Tera::new(&glob) {
            Ok(t) => {
                info!("Loaded templates from {}", config.template);
                Some(Arc::new(t))
            }
            Err(e) => {
                tracing::warn!("Failed to load templates: {e}");
                None
            }
        }
    } else {
        tracing::warn!(
            "Not configuring default handler: Template not found: {}",
            config.template
        );
        None
    };

    if config.reverse_lookup {
        info!("Enabling reverse lookup");
    }
    if config.port_lookup {
        info!("Enabling port lookup");
    }
    if config.sponsor {
        info!("Enabling sponsor logo");
    }
    if !config.trusted_headers.is_empty() {
        info!(
            "Trusting remote IP from header(s): {}",
            config.trusted_headers.join(", ")
        );
    }
    if config.cache_size > 0 {
        info!("Cache capacity set to {}", config.cache_size);
    }
    if config.profile {
        info!("Enabling profiling handlers");
    }

    let state = AppState {
        config: Arc::new(config.clone()),
        geo: geo.clone(),
        cache: cache.clone(),
        tera,
    };

    // Spawn periodic updater
    if config.update_interval > 0 {
        let interval_hours = config.update_interval;
        let data_dir = config.data_dir.clone();
        let geo_for_updater = geo.clone();
        let cache_for_updater = cache.clone();
        let config_for_updater = config.clone();

        info!("Periodic database update every {interval_hours}h");
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(interval_hours * 3600)).await;
                info!("Starting periodic database update...");

                let updater = DbUpdater::new(data_dir.clone().into(), license_key.clone());
                let result = updater.download_all().await;

                let cp = db_updater::resolve_paths(
                    &config_for_updater.country_db,
                    &result.country_path,
                );
                let cip = db_updater::resolve_paths(
                    &config_for_updater.city_db,
                    &result.city_path,
                );
                let ap = db_updater::resolve_paths(
                    &config_for_updater.asn_db,
                    &result.asn_path,
                );
                let ip = db_updater::resolve_paths(
                    config_for_updater.ip66_db.as_deref().unwrap_or(""),
                    &result.ip66_path,
                );

                let new_provider = db_updater::build_provider(
                    cp.as_deref(),
                    cip.as_deref(),
                    ap.as_deref(),
                    ip.as_deref(),
                );

                geo_for_updater.swap(new_provider);
                info!("Geo provider hot-reloaded");

                // Clear cache to avoid serving stale geo data
                {
                    let mut c = cache_for_updater.write().await;
                    *c = Cache::new(c.capacity());
                }
                info!("Cache cleared after database update");
            }
        });
    }

    let app = build_router(state);

    let listen_addr = config.listen_addr();
    info!("Listening on http://{listen_addr}");

    let listener = tokio::net::TcpListener::bind(&listen_addr)
        .await
        .expect("failed to bind");

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await
    .expect("server error");
}
