use std::io::{Read, Write};
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use n0_future::time::Instant;

pub static NON_COMPRESSIBLE_EXTENSIONS: &[&str] = &[
    "3gp",
    "7z",
    "aac",
    "alac",
    "aiff",
    "apk",
    "avi",
    "avif",
    "bin",
    "bmp",
    "bz2",
    "class",
    "cpio",
    "db",
    "dbf",
    "deb",
    "dll",
    "dmg",
    "docx",
    "enc",
    "exe",
    "exr",
    "fbx",
    "flac",
    "flv",
    "gif",
    "glb",
    "gltf",
    "gpg",
    "gz",
    "heic",
    "ico",
    "img",
    "iso",
    "jpeg",
    "jpg",
    "jar",
    "m4a",
    "m4v",
    "mkv",
    "mov",
    "msi",
    "o",
    "obj",
    "ods",
    "odt",
    "ogg",
    "ogv",
    "opus",
    "pdf",
    "pgp",
    "png",
    "pptx",
    "psd",
    "rar",
    "rpm",
    "so",
    "sqlite",
    "sqlite3",
    "tar",
    "tar.bz2",
    "tar.gz",
    "tar.xz",
    "tif",
    "tiff",
    "webm",
    "webp",
    "wmv",
    "wma",
    "xlsx",
    "xz",
    "zip",
    "lz",
    "lzma",
    "zst",
    "zstd",
    "svg",
    "wasm",
    "ipa"
];

pub fn is_compressible(file_name: &str) -> bool {
    let file_name = file_name.to_lowercase();
    !NON_COMPRESSIBLE_EXTENSIONS.iter().any(|ext| file_name.ends_with(ext))
}

pub fn compress(data: &[u8], compressed: bool) -> anyhow::Result<(Vec<u8>, usize, u64)> {
    let raw_len = data.len();
    let mut compression_time = Instant::now();
    let mut buf = Vec::new();
    if !compressed {
        buf.push(0u8);
        buf.extend_from_slice(data);
    } else {
        buf.push(1u8);
        let mut encoder = FrameEncoder::new(vec![]);
        encoder.write_all(data)?;
        let compressed = encoder.finish()?;
        buf.extend_from_slice(&compressed);
    }

    let len = buf.len() as u32;
    let mut out = len.to_le_bytes().to_vec();
    out.extend_from_slice(&buf);
    let compression_time = compression_time.elapsed();
    Ok((out, raw_len, compression_time.as_millis() as u64))
}

pub fn decompress(encoded: &[u8]) -> anyhow::Result<(Vec<u8>, usize)> {
    if encoded.len() < 4 {
        anyhow::bail!("stream too short for length header");
    }
    let len = u32::from_le_bytes(encoded[0..4].try_into().unwrap_or_default()) as usize;
    if encoded.len() < 4 + len {
        anyhow::bail!("stream too short for chunk");
    }
    let chunk = &encoded[4..4 + len];
    let decoded = match chunk[0] {
        0 => chunk[1..].to_vec(), // raw
        1 => {
            let mut decoder = FrameDecoder::new(&chunk[1..]);
            let mut out = Vec::new();
            decoder.read_to_end(&mut out)?;
            out
        }
        _ => anyhow::bail!("unknown chunk type"),
    };

    Ok((decoded, 4 + len))
}

/// Decide whether to compress a chunk based on formula
///
/// # Arguments
/// * `chunk_size` - size of the chunk in bytes
/// * `compression_time_ms` - time it took to compress this chunk in milliseconds
/// * `compressed_size` - resulting compressed size in bytes
/// * `network_bandwidth_bps` - estimated network bandwidth in bytes/sec
///
/// # Returns
/// * `bool` - true if compression is worth it
pub fn should_compress(
    chunk_size: usize,
    compression_time_ms: u64,
    compressed_size: usize,
    network_bandwidth_bps: f64,
) -> bool {
    if network_bandwidth_bps <= 0.0 {
        // bandwidth unknown, default to compress
        return true;
    }

    let ratio = compressed_size as f64 / chunk_size as f64;

    // Skip compression if savings is too small (<5%)
    if ratio > 0.95 {
        return false;
    }

    let t_comp = compression_time_ms as f64 / 1000.0; // convert ms -> s
    let t_send_compressed = compressed_size as f64 / network_bandwidth_bps;
    let t_send_raw = chunk_size as f64 / network_bandwidth_bps;

    // Only compress if total time is less than sending raw
    (t_comp + t_send_compressed) < t_send_raw
}
