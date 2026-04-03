use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

fn is_allowed_source(peer_ip: &IpAddr, addr: SocketAddr) -> bool {
    peer_ip == &addr.ip()
}

#[test]
fn test_ip_filtering_allowed() {
    let peer_ip: IpAddr = Ipv4Addr::new(192, 168, 1, 100).into();
    let source = SocketAddr::new(peer_ip, 8080);

    assert!(is_allowed_source(&peer_ip, source));
}

#[test]
fn test_ip_filtering_denied() {
    let peer_ip: IpAddr = Ipv4Addr::new(192, 168, 1, 100).into();
    let other_ip: IpAddr = Ipv4Addr::new(192, 168, 1, 200).into();
    let source = SocketAddr::new(other_ip, 8080);

    assert!(!is_allowed_source(&peer_ip, source));
}

#[test]
fn test_ip_filtering_ipv6() {
    let peer_ip: IpAddr = Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1).into();
    let source = SocketAddr::new(peer_ip, 8080);

    assert!(is_allowed_source(&peer_ip, source));
}
