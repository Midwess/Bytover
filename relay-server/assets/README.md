# Relay Server GeoIP Assets

The relay-server uses MaxMind's **GeoLite2-Country** database to map its
STUN-discovered public IPv4 to a bytover region (`asia` / `us` / `eu`). The
database is loaded once at startup (see `src/geoip.rs`).

## Files

| File | Required? | Source |
|------|-----------|--------|
| `GeoLite2-Country.mmdb` | vendored (committed to repo) | MaxMind GeoLite2 |
| `LICENSE-GEOLITE2.txt` | required when distributing the DB | MaxMind |

## Vendored database

`GeoLite2-Country.mmdb` is **committed to the repo** and copied into the
Docker image at `/app/assets/GeoLite2-Country.mmdb`. This keeps builds
hermetic — no MaxMind account, license key, or network access is needed at
build time.

The current copy was sourced from the sibling `rpc-signalling` project,
which uses the same MaxMind DB for the same purpose.

## Refreshing the database

MaxMind publishes updates roughly weekly. Country-level IP allocations are
stable, so refreshing every few months is sufficient. To refresh:

1. Sign up at <https://www.maxmind.com/en/geolite2/signup> (free account).
2. Generate a license key in the account portal.
3. Download:

   ```bash
   curl -L -o GeoLite2-Country.tar.gz \
     "https://download.maxmind.com/app/geoip_download?edition_id=GeoLite2-Country&license_key=$MAXMIND_LICENSE_KEY&suffix=tar.gz"
   tar --strip-components=1 -xzf GeoLite2-Country.tar.gz \
     '*/GeoLite2-Country.mmdb' '*/LICENSE.txt'
   mv LICENSE.txt LICENSE-GEOLITE2.txt
   rm GeoLite2-Country.tar.gz
   ```

4. Replace `GeoLite2-Country.mmdb` and `LICENSE-GEOLITE2.txt` in this directory.
5. Commit the updated files.

## Override env

| Env | Effect |
|-----|--------|
| `BYTOVER_REGION_CODE` | Skip both GeoIP and gRPC; use this region directly. |
| `BYTOVER_GEOIP_DB_PATH` | Override the `.mmdb` path. Default: `/app/assets/GeoLite2-Country.mmdb`. |

## Failure mode

If the DB file is missing or unreadable at startup, the relay logs a
`WARN` and falls back to the gRPC `GetRegion` path. It does not panic.

## License attribution

Distributing the GeoLite2 database requires the bundled `LICENSE-GEOLITE2.txt`
to be retained alongside the data, and attribution displayed where reasonable.
The CC BY-SA 4.0 license requires:

> This product includes GeoLite2 data created by MaxMind, available from
> <https://www.maxmind.com>.
