# echoip

A Rust port of [mpolden/echoip](https://github.com/mpolden/echoip) — a simple service for looking up your IP address.

Supports both [MaxMind GeoIP2](https://www.maxmind.com) and [ip66.dev](https://ip66.dev/) as geolocation data sources.

## Usage

```
$ curl ifconfig.co
127.0.0.1

$ curl ifconfig.co/country
Elbonia

$ curl ifconfig.co/country-iso
EB

$ curl ifconfig.co/city
Bornyasherk

$ curl ifconfig.co/asn
AS31337

$ curl ifconfig.co/asn-org
Dilbert Technologies
```

As JSON:

```
$ curl -H 'Accept: application/json' ifconfig.co  # or curl ifconfig.co/json
{
  "city": "Bornyasherk",
  "country": "Elbonia",
  "country_iso": "EB",
  "ip": "127.0.0.1",
  "ip_decimal": 2130706433,
  "asn": "AS31337",
  "asn_org": "Dilbert Technologies"
}
```

Port testing:

```
$ curl ifconfig.co/port/80
{
  "ip": "127.0.0.1",
  "port": 80,
  "reachable": false
}
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

## Geolocation Data

### MaxMind GeoIP2

Download the MaxMind GeoLite2 databases:

```
GEOIP_LICENSE_KEY=<key> make geoip-download
```

Requires a [MaxMind account and license key](https://dev.maxmind.com/geoip/geolite2-free-geolocation-data).

### ip66.dev

No setup required. Enable with the `--ip66` flag. Optionally set a custom API URL with `--ip66-url`.

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
      --ip66                             Enable ip66.dev as geo provider
      --ip66-url <IP66_URL>              Custom ip66.dev API base URL
  -h, --help                             Print help
```

## Examples

Using MaxMind only:

```
echoip -f GeoLite2-Country.mmdb -c GeoLite2-City.mmdb -a GeoLite2-ASN.mmdb -r -p
```

Using ip66.dev only (no local databases needed):

```
echoip --ip66 -r -p
```

Using both (MaxMind primary, ip66.dev fallback):

```
echoip -f GeoLite2-Country.mmdb -c GeoLite2-City.mmdb -a GeoLite2-ASN.mmdb --ip66 -r -p
```

## License

BSD 3-Clause. Based on [mpolden/echoip](https://github.com/mpolden/echoip).
