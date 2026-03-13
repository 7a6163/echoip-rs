# Changelog

## [1.0.0] - 2026-03-13

### Added

- Rust port of [mpolden/echoip](https://github.com/mpolden/echoip) using Axum
- IPv4 and IPv6 support with automatic CLI client detection
- JSON, plain text, and HTML response formats with content negotiation
- MaxMind GeoIP2 integration (Country, City, ASN databases)
- ip66.dev MMDB integration as alternative/fallback geo provider
- Composite geo provider with pluggable primary/fallback strategy
- Auto-download of GeoIP databases via environment variables
  - New MaxMind API (`MAXMIND_ACCOUNT_ID` + `MAXMIND_LICENSE_KEY`)
  - Legacy MaxMind API (`GEOIP_LICENSE_KEY`)
  - ip66.dev (no key required)
- Periodic database updates with hot-reload (no restart needed)
- LRU response cache with configurable capacity
- Port reachability testing
- Custom IP lookup via `?ip=` query parameter
- Reverse DNS hostname lookup
- Trusted proxy header support (`-H` flag)
- HTML interface with OpenStreetMap and dark/light theme
- Multi-arch Docker image (amd64 + arm64)
- CI pipeline with formatting, clippy, tests, and code coverage
- Automated release workflow (crates.io + Docker)
