# Relay Server GeoIP Assets

The relay-server uses MaxMind's **GeoLite2-Country** database to map its
STUN-discovered public IPv4 to a bytover region (`asia` / `us` / `eu`). The
database is loaded once at startup (see `src/geoip.rs`).

## Files

| File | Required? | Source |
|------|-----------|--------|
| `GeoLite2-Country.mmdb` | optional (resolver gracefully degrades to gRPC) | MaxMind |
| `LICENSE-GEOLITE2.txt` | required when distributing the DB | MaxMind |

## Acquiring the database

GeoLite2 is free under **CC BY-SA 4.0** but requires a (free) MaxMind
account and license key.

1. Sign up at <https://www.maxmind.com/en/geolite2/signup>
2. Generate a license key in the account portal
3. Download the country DB:

   ```bash
   curl -L -o GeoLite2-Country.tar.gz \
     "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=$MAXMIND_LICENSE_KEY&suffix=tar.gz"
   tar --strip-components=1 -xzf GeoLite2-Country.tar.gz \
     '*/GeoLite2-Country.mmdb' '*/LICENSE.txt'
   mv LICENSE.txt LICENSE-GEOLITE2.txt
   rm GeoLite2-Country.tar.gz
   ```

4. Place `GeoLite2-Country.mmdb` and `LICENSE-GEOLITE2.txt` in this directory.

## Build-time vs runtime

- **Image build**: the Dockerfile copies `assets/*` into `/app/assets/`. If
  `GeoLite2-Country.mmdb` is missing at build time, the image still builds and
  the relay falls back to the gRPC `GetRegion` path with a `WARN` log at
  startup.
- **Override at runtime**: set `BYTOVER_GEOIP_DB_PATH=/path/inside/container`
  to point at a sidecar-mounted DB.

## Override env

| Env | Effect |
|-----|--------|
| `BYTOVER_REGION_CODE` | Skip both GeoIP and gRPC; use this region directly. |
| `BYTOVER_GEOIP_DB_PATH` | Override the `.mmdb` path. Default: `/app/assets/GeoLite2-Country.mmdb`. |

## License attribution

Distributing the GeoLite2 database requires the bundled `LICENSE-GEOLITE2.txt`
to be retained alongside the data, and attribution displayed where reasonable.
The CC BY-SA 4.0 license requires:

> This product includes GeoLite2 data created by MaxMind, available from
> <https://www.maxmind.com>.

Place the upstream `LICENSE.txt` here as `LICENSE-GEOLITE2.txt` to satisfy this.
