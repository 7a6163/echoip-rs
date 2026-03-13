use axum::extract::State;
use axum::http::HeaderMap;
use axum::response::IntoResponse;

use crate::error::AppError;
use crate::server::AppState;

pub async fn html_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: axum::extract::Query<std::collections::HashMap<String, String>>,
    remote: axum::extract::ConnectInfo<std::net::SocketAddr>,
) -> Result<impl IntoResponse, AppError> {
    let tera = state
        .tera
        .as_ref()
        .ok_or_else(|| AppError::internal("templates not configured"))?;

    let query_ip = query.get("ip").map(|s| s.as_str());
    let resp = crate::server::build_response(&state, &headers, query_ip, Some(remote.0.ip())).await?;

    let json_str = serde_json::to_string_pretty(&resp)
        .map_err(|e| AppError::internal(e.to_string()))?;

    let mut ctx = tera::Context::new();
    ctx.insert("IP", &resp.ip.to_string());
    ctx.insert("IPDecimal", &resp.ip_decimal.to_string());
    ctx.insert("Country", &resp.country);
    ctx.insert("CountryISO", &resp.country_iso);
    ctx.insert("CountryEU", &resp.country_eu);
    ctx.insert("RegionName", &resp.region_name);
    ctx.insert("RegionCode", &resp.region_code);
    ctx.insert("MetroCode", &resp.metro_code);
    ctx.insert("PostalCode", &resp.zip_code);
    ctx.insert("City", &resp.city);
    ctx.insert("Latitude", &resp.latitude);
    ctx.insert("Longitude", &resp.longitude);
    ctx.insert("Timezone", &resp.time_zone);
    ctx.insert("ASN", &resp.asn);
    ctx.insert("ASNOrg", &resp.asn_org);
    ctx.insert("Hostname", &resp.hostname);
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    ctx.insert("Host", host);
    ctx.insert("BoxLatTop", &(resp.latitude + 0.05));
    ctx.insert("BoxLatBottom", &(resp.latitude - 0.05));
    ctx.insert("BoxLonLeft", &(resp.longitude - 0.05));
    ctx.insert("BoxLonRight", &(resp.longitude + 0.05));
    ctx.insert("JSON", &json_str);
    ctx.insert("Port", &state.config.port_lookup);
    ctx.insert("Sponsor", &state.config.sponsor);
    ctx.insert("ExplicitLookup", &query.contains_key("ip"));

    let html = tera
        .render("index.html", &ctx)
        .map_err(|e| AppError::internal(e.to_string()))?;

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/html; charset=utf-8".to_string(),
        )],
        html,
    ))
}
