# Changelog

## [1.1.0] - 2026-03-13

### Changed

- Remove `async_trait` dependency — use native async traits (Rust 2024 edition)
- Remove `fnv` dependency — use `IpAddr` directly as LRU cache key
- GeoIP lookups changed from async to sync (MMDB reads are memory-mapped, no async needed)
- Rename `as_json()` to `into_json()` for idiomatic Rust naming
- Extract `attach_user_agent()` helper to reduce code duplication
- `--cache-size 0` now truly disables caching (was silently creating capacity=1)
- Cache `get()` uses write lock for proper LRU access tracking

### Added

- Graceful shutdown on SIGTERM/SIGINT (Docker-friendly)
- HTTP request tracing via `TraceLayer`
- DNS reverse lookup timeout (3 seconds)
- MSRV policy: Rust 1.85+ (`rust-version` in Cargo.toml)
- CI: MSRV check, `cargo audit` security audit, clippy/fmt in release workflow
- Dockerfile: dependency caching layer, HEALTHCHECK directive
- Unit tests for `response.rs`, `error.rs`, `config.rs`, `ip_util.rs` extract_ip
- Integration tests: HEAD request, `?ip=` override, IPv6, cache hit/miss, cache disabled, trusted headers, X-Forwarded-For

### Fixed

- `--cache-size 0` created a cache with capacity 1 instead of disabling
- Test server ignored config's `cache_size`, always used 100

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
