use std::path::Path;
use std::sync::Arc;

use clap::Parser;
use tokio::sync::RwLock;
use tracing::info;

use echoip::cache::Cache;
use echoip::config::Config;
use echoip::geo::composite::CompositeProvider;
use echoip::geo::ip66::Ip66Provider;
use echoip::geo::maxmind::MaxmindProvider;
use echoip::geo::GeoProvider;
use echoip::server::{build_router, AppState};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = Config::parse();

    // Build geo providers
    let mut providers: Vec<Box<dyn GeoProvider>> = Vec::new();

    // MaxMind provider (if any DB paths are specified)
    let country_db = if config.country_db.is_empty() {
        None
    } else {
        Some(config.country_db.as_str())
    };
    let city_db = if config.city_db.is_empty() {
        None
    } else {
        Some(config.city_db.as_str())
    };
    let asn_db = if config.asn_db.is_empty() {
        None
    } else {
        Some(config.asn_db.as_str())
    };

    if country_db.is_some() || city_db.is_some() || asn_db.is_some() {
        match MaxmindProvider::open(country_db, city_db, asn_db) {
            Ok(provider) => {
                info!("Loaded MaxMind GeoIP databases");
                providers.push(Box::new(provider));
            }
            Err(e) => {
                tracing::error!("Failed to open MaxMind databases: {e}");
                std::process::exit(1);
            }
        }
    }

    // ip66.dev provider
    if config.ip66 {
        info!("Enabling ip66.dev geo provider");
        providers.push(Box::new(Ip66Provider::new(config.ip66_url.clone())));
    }

    let geo: Arc<dyn GeoProvider> = if providers.is_empty() {
        // Empty provider that returns nothing
        Arc::new(MaxmindProvider::open(None, None, None).unwrap())
    } else if providers.len() == 1 {
        Arc::from(providers.into_iter().next().unwrap())
    } else {
        Arc::new(CompositeProvider::new(providers))
    };

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
        geo,
        cache,
        tera,
    };

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
