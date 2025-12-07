use crate::protocol::webrtc::webrtc::MAX_BUFFER_SIZE;

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
    "wasm",
    "ipa"
];

pub fn is_compressible(file_name: &str) -> bool {
    let file_name = file_name.to_lowercase();
    !NON_COMPRESSIBLE_EXTENSIONS.iter().any(|ext| file_name.ends_with(ext))
}

pub struct CompressStats {
    chunk_size: usize,
    compression_time_micro: u64,
    compressed_size: usize,
    network_bandwidth_bps: f64,
    read_time_micro: u64,

    should_compress: bool,
    failed_bytes: usize,
}

impl CompressStats {
    pub fn new() -> Self {
        Self { chunk_size: 0, compression_time_micro: 0, compressed_size: 0, network_bandwidth_bps: 0.0, read_time_micro: 0, failed_bytes: 0, should_compress: true }
    }

    pub fn add_chunk_stats(&mut self, raw_size: usize, compression_time_micro: u64, compressed_size: usize, read_time_micro: u64) {
        self.chunk_size += raw_size;
        self.compression_time_micro += compression_time_micro;
        self.compressed_size += compressed_size;
        self.read_time_micro += read_time_micro;
        if compressed_size > raw_size {
            self.failed_bytes += raw_size;
        }
        else {
            self.failed_bytes = 0;
        }

        self.should_compress = self.cal_should_compress();
    }

    pub fn update_network_bandwidth(&mut self, network_bandwidth_bps: f64) {
        self.network_bandwidth_bps = network_bandwidth_bps;
        self.should_compress = self.cal_should_compress();
    }

    pub fn new_round(&mut self) {
        // We reset everything except network bandwidth,
        // because it is already bandwidth at specific time.
        self.chunk_size = 0;
        self.compression_time_micro = 0;
        self.compressed_size = 0;
        self.read_time_micro = 0;
        self.failed_bytes = 0;
    }

    pub fn should_compress(&self) -> bool {
        self.should_compress
    }

    fn cal_should_compress(&self) -> bool {
        if self.failed_bytes > MAX_BUFFER_SIZE {
            return false;
        }

        if self.chunk_size <= self.compressed_size {
            return false;
        }

        let read_time_seconds = self.read_time_micro as f64 / 1_000_000.0;
        let disk_bandwidth_bps = self.chunk_size as f64 / read_time_seconds;

        if self.network_bandwidth_bps <= 0.0 || disk_bandwidth_bps <= 0.0 {
            return true; // fallback: compress if we don't know speed
        }

        // Don't compress if compression ratio is too small
        let ratio = self.compressed_size as f64 / self.chunk_size as f64;
        if ratio > 0.94 {
            return false;
        }

        let effective_bw = self.network_bandwidth_bps.min(disk_bandwidth_bps);

        let t_comp = self.compression_time_micro as f64 / 1_000_000.0;
        let t_send_compressed = self.compressed_size as f64 / effective_bw;
        let t_send_raw = self.chunk_size as f64 / effective_bw;

        // There is no way we are able to calculate correct 100% network speed.
        // I add this ratio to make sure the compression must have significant impact on network speed.
        let imo = 0.95;
        (t_comp + t_send_compressed) < t_send_raw * imo
    }
}
