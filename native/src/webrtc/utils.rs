pub fn sanitize_sdp(sdp: &str) -> String {
    sdp.lines()
        .filter(|line| {
            if (line.contains("candidate:") || line.contains("candidate:")) && line.contains(".local") {
                log::info!("[webrtc] Stripping mDNS candidate from SDP: {}", line);
                false
            } else {
                true
            }
        })
        .collect::<Vec<_>>()
        .join("\r\n")
}
