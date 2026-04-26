use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::path::{Path, PathBuf};

use maxminddb::geoip2;

pub struct GeoipResolver {
    reader: maxminddb::Reader<Vec<u8>>,
    db_path: PathBuf,
}

impl GeoipResolver {
    pub fn load(path: &Path) -> Result<Self, String> {
        let reader = maxminddb::Reader::open_readfile(path)
            .map_err(|error| format!("failed to open GeoIP database at {}: {error}", path.display()))?;
        Ok(Self {
            reader,
            db_path: path.to_path_buf(),
        })
    }

    pub fn db_path(&self) -> &Path {
        &self.db_path
    }

    pub fn region_for(&self, addr: IpAddr) -> Option<&'static str> {
        if !is_routable_global(addr) {
            return None;
        }
        let country: geoip2::Country = self.reader.lookup(addr).ok()?;
        let iso = country.country.and_then(|c| c.iso_code)?;
        country_to_region(iso)
    }
}

pub fn country_to_region(iso_code: &str) -> Option<&'static str> {
    let upper: [u8; 2] = match iso_code.as_bytes() {
        [a, b] => [a.to_ascii_uppercase(), b.to_ascii_uppercase()],
        _ => return None,
    };
    match &upper {
        b"US" | b"CA" | b"MX" => Some("us"),
        b"GB" | b"IE" | b"DE" | b"FR" | b"IT" | b"ES" | b"PT" | b"NL" | b"BE" | b"LU"
        | b"AT" | b"CH" | b"SE" | b"NO" | b"FI" | b"DK" | b"IS" | b"PL" | b"CZ" | b"SK"
        | b"HU" | b"RO" | b"BG" | b"GR" | b"EE" | b"LV" | b"LT" | b"SI" | b"HR" | b"RS"
        | b"UA" => Some("eu"),
        b"SG" | b"JP" | b"KR" | b"HK" | b"TW" | b"CN" | b"IN" | b"ID" | b"VN" | b"TH"
        | b"MY" | b"PH" | b"BD" | b"PK" | b"LK" | b"AU" | b"NZ" => Some("asia"),
        _ => None,
    }
}

fn is_routable_global(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => is_routable_global_v4(v4),
        IpAddr::V6(v6) => is_routable_global_v6(v6),
    }
}

fn is_routable_global_v4(ipv4: Ipv4Addr) -> bool {
    if ipv4.is_loopback() || ipv4.is_private() || ipv4.is_link_local() || ipv4.is_broadcast() || ipv4.is_unspecified() || ipv4.is_documentation() {
        return false;
    }
    let octets = ipv4.octets();
    if octets[0] == 100 && (octets[1] & 0xC0) == 0x40 {
        return false;
    }
    if octets[0] == 192 && octets[1] == 0 && octets[2] == 0 {
        return false;
    }
    if octets[0] == 198 && (octets[1] == 18 || octets[1] == 19) {
        return false;
    }
    if octets[0] >= 240 {
        return false;
    }
    true
}

fn is_routable_global_v6(ipv6: Ipv6Addr) -> bool {
    if ipv6.is_loopback() || ipv6.is_unspecified() || ipv6.is_multicast() {
        return false;
    }
    let segs = ipv6.segments();
    if (segs[0] & 0xfe00) == 0xfc00 {
        return false;
    }
    if (segs[0] & 0xffc0) == 0xfe80 {
        return false;
    }
    if segs[0] == 0x2001 && segs[1] == 0x0db8 {
        return false;
    }
    if segs[0] == 0x0100 && segs[1] == 0 && segs[2] == 0 && segs[3] == 0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::{country_to_region, is_routable_global, GeoipResolver};
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::path::{Path, PathBuf};

    #[test]
    fn americas_codes_map_to_us() {
        assert_eq!(country_to_region("US"), Some("us"));
        assert_eq!(country_to_region("CA"), Some("us"));
        assert_eq!(country_to_region("MX"), Some("us"));
    }

    #[test]
    fn europe_codes_map_to_eu() {
        assert_eq!(country_to_region("GB"), Some("eu"));
        assert_eq!(country_to_region("DE"), Some("eu"));
        assert_eq!(country_to_region("FR"), Some("eu"));
        assert_eq!(country_to_region("UA"), Some("eu"));
    }

    #[test]
    fn asia_codes_map_to_asia() {
        assert_eq!(country_to_region("SG"), Some("asia"));
        assert_eq!(country_to_region("JP"), Some("asia"));
        assert_eq!(country_to_region("AU"), Some("asia"));
        assert_eq!(country_to_region("IN"), Some("asia"));
    }

    #[test]
    fn unmapped_country_returns_none() {
        assert_eq!(country_to_region("ZA"), None);
        assert_eq!(country_to_region("BR"), None);
        assert_eq!(country_to_region("RU"), None);
    }

    #[test]
    fn lowercase_iso_is_normalized() {
        assert_eq!(country_to_region("sg"), Some("asia"));
        assert_eq!(country_to_region("us"), Some("us"));
    }

    #[test]
    fn malformed_iso_returns_none() {
        assert_eq!(country_to_region(""), None);
        assert_eq!(country_to_region("S"), None);
        assert_eq!(country_to_region("SGP"), None);
    }

    #[test]
    fn loopback_and_private_skip_geoip() {
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::LOCALHOST)));
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))));
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))));
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::new(172, 16, 0, 1))));
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::new(100, 64, 0, 1))));
        assert!(!is_routable_global(IpAddr::V4(Ipv4Addr::new(169, 254, 0, 1))));
    }

    #[test]
    fn public_ipv4_is_routable() {
        assert!(is_routable_global(IpAddr::V4(Ipv4Addr::new(64, 118, 143, 14))));
        assert!(is_routable_global(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))));
    }

    #[test]
    fn loopback_and_reserved_v6_skip_geoip() {
        assert!(!is_routable_global(IpAddr::V6(Ipv6Addr::LOCALHOST)));
        assert!(!is_routable_global(IpAddr::V6(Ipv6Addr::UNSPECIFIED)));
        assert!(!is_routable_global(IpAddr::V6("ff02::1".parse().unwrap())));
        assert!(!is_routable_global(IpAddr::V6("fc00::1".parse().unwrap())));
        assert!(!is_routable_global(IpAddr::V6("fd12:3456:789a::1".parse().unwrap())));
        assert!(!is_routable_global(IpAddr::V6("fe80::1".parse().unwrap())));
        assert!(!is_routable_global(IpAddr::V6("2001:db8::1".parse().unwrap())));
        assert!(!is_routable_global(IpAddr::V6("100::1".parse().unwrap())));
    }

    #[test]
    fn public_ipv6_is_routable() {
        assert!(is_routable_global(IpAddr::V6("2404:c140:2100:6::46:6f69".parse().unwrap())));
        assert!(is_routable_global(IpAddr::V6("2606:4700:4700::1111".parse().unwrap())));
    }

    #[test]
    fn missing_db_returns_err() {
        let result = GeoipResolver::load(Path::new("/nonexistent/path/GeoLite2-Country.mmdb"));
        match result {
            Ok(_) => panic!("expected Err for nonexistent path"),
            Err(error) => assert!(error.contains("failed to open GeoIP database"), "unexpected error: {error}"),
        }
    }

    fn bundled_mmdb_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/GeoLite2-Country.mmdb")
    }

    #[test]
    fn singapore_host_v6_resolves_to_asia() {
        let resolver = GeoipResolver::load(&bundled_mmdb_path()).expect("load bundled mmdb");
        let v6: Ipv6Addr = "2404:c140:2100:6::46:6f69".parse().unwrap();
        assert_eq!(resolver.region_for(IpAddr::V6(v6)), Some("asia"));
    }

    #[test]
    fn singapore_host_v4_misclassified_as_us_in_geolite2() {
        let resolver = GeoipResolver::load(&bundled_mmdb_path()).expect("load bundled mmdb");
        assert_eq!(
            resolver.region_for(IpAddr::V4(Ipv4Addr::new(64, 118, 143, 14))),
            Some("us")
        );
    }
}
