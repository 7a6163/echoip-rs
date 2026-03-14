# echoip

[![CI](https://github.com/7a6163/echoip-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/7a6163/echoip-rs/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/7a6163/echoip-rs/graph/badge.svg?token=2C8PU0G22O)](https://codecov.io/gh/7a6163/echoip-rs)

A Rust port of [mpolden/echoip](https://github.com/mpolden/echoip) — a simple service for looking up your IP address.

Supports both [MaxMind GeoIP2](https://www.maxmind.com) and [ip66.dev](https://ip66.dev/) as geolocation data sources.

## Usage

```
$ curl localhost:8080
1.2.3.4

$ curl localhost:8080/country
United States

$ curl localhost:8080/country-iso
US

$ curl localhost:8080/city
San Francisco

$ curl localhost:8080/asn
AS13335

$ curl localhost:8080/asn-org
Cloudflare, Inc.
```

As JSON:

```
$ curl localhost:8080/json  # or curl -H 'Accept: application/json' localhost:8080
{
  "ip": "1.2.3.4",
  "ip_decimal": 16909060,
  "country": "United States",
  "country_iso": "US",
  "city": "San Francisco",
  "latitude": 37.7749,
  "longitude": -122.4194,
  "time_zone": "America/Los_Angeles",
  "asn": "AS13335",
  "asn_org": "Cloudflare, Inc."
}
```

Port testing:

```
$ curl localhost:8080/port/443
{
  "ip": "1.2.3.4",
  "port": 443,
  "reachable": true
}
```

Custom IP lookup:

```
$ curl localhost:8080/json?ip=8.8.8.8
```

Pass `-4` or `-6` to your client to switch between IPv4 and IPv6 lookup.

## Features

- Supports IPv4 and IPv6
- Supports common CLI clients (curl, wget, httpie, fetch, xh)
- JSON output with geolocation, ASN, reverse DNS, and user agent info
- Country, city, ASN lookup via MaxMind GeoIP2 or ip66.dev
- Pluggable geo provider with automatic fallback (MaxMind primary, ip66.dev fallback, or vice versa)
- Port reachability testing
- Custom IP lookup via `?ip=` query parameter (all endpoints except `/port`)
- LRU response cache
- Auto-download databases on startup via `GEOIP_LICENSE_KEY` env var
- Periodic database updates with hot-reload (no restart needed)
- HTML interface with OpenStreetMap and dark/light theme
- Docker support

## Building

Requires [Rust](https://www.rust-lang.org/tools/install) 1.85+.

```
cargo build --release
```

Or install directly:

```
cargo install --path .
```

## Docker

```
docker build -t echoip .
docker run -p 8080:8080 echoip
```

The Docker image uses [distroless](https://github.com/GoogleContainerTools/distroless) (~52MB) for minimal attack surface. Use orchestrator-level health checks (e.g. Kubernetes liveness probe) against `/health`.

## Geolocation Data

### Automatic Download (Recommended)

Set environment variables and databases will be downloaded automatically on startup. Both old and new MaxMind API formats are supported.

**New API** (Account ID + License Key, recommended):

```
MAXMIND_ACCOUNT_ID=<id> MAXMIND_LICENSE_KEY=<key> echoip -r -p
```

**Legacy API** (License Key only):

```
GEOIP_LICENSE_KEY=<key> echoip -r -p
```

This downloads MaxMind GeoLite2 (Country, City, ASN) and ip66.dev databases to `data/`. ip66.dev requires no key and is always downloaded.

For periodic updates (e.g. every 24 hours):

```
MAXMIND_ACCOUNT_ID=<id> MAXMIND_LICENSE_KEY=<key> echoip -r -p --update-interval 24
```

Databases are hot-reloaded without restarting the server.

| Environment Variable | Description |
|---------------------|-------------|
| `MAXMIND_ACCOUNT_ID` | MaxMind account ID (new API) |
| `MAXMIND_LICENSE_KEY` | MaxMind license key (new API) |
| `GEOIP_LICENSE_KEY` | MaxMind license key (legacy API, used when `MAXMIND_ACCOUNT_ID` is not set) |

### Manual Download

ip66.dev (free, no account):

```
curl -LO https://downloads.ip66.dev/db/ip66.mmdb
```

MaxMind GeoLite2 requires a [MaxMind account and license key](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data).

## CLI Options

```
$ echoip --help
Usage: echoip [OPTIONS]

Options:
  -f, --country-db <COUNTRY_DB>          Path to GeoIP country database
  -c, --city-db <CITY_DB>                Path to GeoIP city database
  -a, --asn-db <ASN_DB>                  Path to GeoIP ASN database
  -l, --listen <LISTEN>                  Listening address [default: :8080]
  -r, --reverse-lookup                   Perform reverse hostname lookups
  -p, --port-lookup                      Enable port lookup
  -t, --template <TEMPLATE>              Path to template directory [default: html]
  -C, --cache-size <CACHE_SIZE>          Size of response cache (0 to disable) [default: 0]
  -P, --profile                          Enable profiling/debug handlers
  -s, --sponsor                          Show sponsor logo
  -H, --trusted-header <TRUSTED_HEADER>  Headers to trust for remote IP (repeatable)
      --ip66-db <IP66_DB>                Path to ip66.dev MMDB database
  -d, --data-dir <DATA_DIR>              Directory for auto-downloaded databases [default: data]
      --update-interval <HOURS>          Auto-update interval in hours (0 to disable) [default: 0]
      --no-auto-download                 Disable automatic database download on startup
  -h, --help                             Print help
```

## Examples

Auto-download and start (new API):

```
MAXMIND_ACCOUNT_ID=<id> MAXMIND_LICENSE_KEY=<key> echoip -r -p
```

Auto-download and start (legacy API):

```
GEOIP_LICENSE_KEY=<key> echoip -r -p
```

Auto-download with periodic updates every 24 hours:

```
MAXMIND_ACCOUNT_ID=<id> MAXMIND_LICENSE_KEY=<key> echoip -r -p --update-interval 24
```

Manual database paths:

```
echoip -f GeoLite2-Country.mmdb -c GeoLite2-City.mmdb -a GeoLite2-ASN.mmdb --ip66-db ip66.mmdb -r -p --no-auto-download
```

ip66.dev only (no MaxMind key needed):

```
echoip --ip66-db ip66.mmdb -r -p --no-auto-download
```

## License

BSD 3-Clause. Based on [mpolden/echoip](https://github.com/mpolden/echoip).
