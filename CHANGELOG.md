# Changelog

## [1.4.1] - 2026-03-31

### Fixed

- Add DB-IP attribution to map footer (required by DB-IP Lite license)

## [1.4.0] - 2026-03-26

### Added

- DB-IP Lite City as default free geolocation database (country + city + coordinates)
- Auto-download DB-IP on startup (no API key required)
- Map/coordinates now work without MaxMind (DB-IP provides latitude/longitude)

### Changed

- Default free provider priority: MaxMind > DB-IP > ip66.dev
- DB-IP is primary free source (has city + coords); ip66.dev is fallback (country + ASN only)

## [1.3.2] - 2026-03-24

### Security

- Update `rustls-webpki` 0.103.9 → 0.103.10 (RUSTSEC-2026-0049: CRL matching bug)
- Update `tar` 0.4.44 → 0.4.45 (RUSTSEC-2026-0067, RUSTSEC-2026-0068: symlink chmod + PAX header)
- Update `lru` 0.12.5 → 0.16.3 (RUSTSEC-2026-0002: IterMut unsoundness)

## [1.3.1] - 2026-03-24

### Fixed

- Widen page layout (880px → 1080px) to prevent content overflow
- IP input field too narrow for full IPv4 addresses (14ch → 20ch)
- Details grid ratio 1:1 → 3:2 for better readability
- Use `word-break: break-word` instead of `break-all` for natural text wrapping

### Added

- Docker Compose examples in README

## [1.3.0] - 2026-03-14

### Changed

- Rewrite let chains to nested if/let for broader Rust version compatibility
- Lower MSRV from 1.87 to 1.85 (edition 2024 minimum)
- CI coverage excludes binary entry point (`main.rs`)

### Added

- Test coverage increased from 40% to 90%+
- Unit tests for geo providers (MaxMind, ip66, composite, swappable)
- Unit tests for db_updater (resolve_paths, build_provider, extract_mmdb, validate_mmdb)
- Integration tests for HTML handler, content negotiation, port handler, error branches
- MaxMind test MMDB fixtures for geo provider testing

### Fixed

- CI workflow branch triggers changed from `master` to `main`

## [1.2.0] - 2026-03-14

### Changed

- Docker image switched from `debian:bookworm-slim` to `gcr.io/distroless/cc-debian13` (126MB → 52MB)
- Removed HEALTHCHECK from Dockerfile (distroless has no shell; use orchestrator-level health checks instead)
- Removed VOLUME directive (mount data directory at runtime)

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
