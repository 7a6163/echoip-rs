DOCKER_IMAGE := mpolden/echoip

all: build

build:
	cargo build --release

test:
	cargo test

vet:
	cargo clippy -- -D warnings

fmt:
	cargo fmt

checkfmt:
	cargo fmt -- --check

install:
	cargo install --path .

run:
	cargo run -- -f /usr/share/GeoIP/GeoLite2-Country.mmdb \
		-c /usr/share/GeoIP/GeoLite2-City.mmdb \
		-a /usr/share/GeoIP/GeoLite2-ASN.mmdb \
		-r -p -s

docker-build:
	docker build -t $(DOCKER_IMAGE) .

docker-test: docker-build
	@docker rm -f echoip-test 2>/dev/null || true
	docker run -d --name echoip-test -p 8081:8080 $(DOCKER_IMAGE)
	sleep 1
	curl -s -4 localhost:8081 | grep -q '.'
	@docker rm -f echoip-test

geoip-download:
	mkdir -p data
	curl -sL "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=$(GEOIP_LICENSE_KEY)&suffix=tar.gz" | tar -xzf - --strip-components=1 -C data
	curl -sL "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-City&license_key=$(GEOIP_LICENSE_KEY)&suffix=tar.gz" | tar -xzf - --strip-components=1 -C data
	curl -sL "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-ASN&license_key=$(GEOIP_LICENSE_KEY)&suffix=tar.gz" | tar -xzf - --strip-components=1 -C data

.PHONY: all build test vet fmt checkfmt install run docker-build docker-test geoip-download
