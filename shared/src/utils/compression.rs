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
    is_compression_support: bool
}

impl CompressStats {
    pub fn new(is_compression_support: bool) -> Self {
        Self { chunk_size: 0, compression_time_micro: 0, compressed_size: 0, network_bandwidth_bps: 0.0, read_time_micro: 0, failed_bytes: 0, should_compress: false, is_compression_support }
    }

    pub fn is_compression_support(&self) -> bool {
        self.is_compression_support
    }

    pub fn add_chunk_stats(&mut self, raw_size: usize, compression_time_micro: u64, compressed_size: usize, read_time_micro: u64) -> bool {
        self.chunk_size += raw_size;
        self.compression_time_micro += compression_time_micro;
        self.compressed_size += compressed_size;
        self.read_time_micro += read_time_micro;
        let is_success = self.should_compress && compressed_size < raw_size;
        if is_success {
            self.failed_bytes = 0;
        }
        else {
            self.failed_bytes += raw_size;
        }

        self.should_compress = self.cal_should_compress();
        is_success
    }

    pub fn update_network_bandwidth(&mut self, network_bandwidth_bps: f64) -> bool {
        if network_bandwidth_bps == self.network_bandwidth_bps {
            return self.should_compress;
        }
        
        self.network_bandwidth_bps = network_bandwidth_bps;
        self.should_compress = self.cal_should_compress();
        self.should_compress
    }

    pub fn start_over(&mut self) {
        self.network_bandwidth_bps = 0.0;
        self.chunk_size = 0;
        self.compression_time_micro = 0;
        self.compressed_size = 0;
        self.read_time_micro = 0;
        self.should_compress = true;
    }

    pub fn should_compress(&self) -> bool {
        self.should_compress && self.is_compression_support
    }

    pub fn no_compress(&mut self) {
        self.should_compress = false;
    }

    fn cal_should_compress(&self) -> bool {
        if self.network_bandwidth_bps <= 1.0 {
            return self.should_compress;
        }

        if self.failed_bytes > MAX_BUFFER_SIZE {
            return false;
        }

        if self.chunk_size <= self.compressed_size {
            return false;
        }

        let read_time_seconds = self.read_time_micro as f64 / 1_000_000.0;
        let disk_bandwidth_bps = self.chunk_size as f64 / read_time_seconds;

        if self.network_bandwidth_bps <= 0.0 || disk_bandwidth_bps <= 0.0 {
            return false;
        }

        let ratio = self.compressed_size as f64 / self.chunk_size as f64;
        if ratio > 0.94 {
            return false;
        }

        let effective_bw = self.network_bandwidth_bps.min(disk_bandwidth_bps);

        let t_comp = self.compression_time_micro as f64 / 1_000_000.0;
        let t_send_compressed = self.compressed_size as f64 / effective_bw;
        let t_send_raw = self.chunk_size as f64 / effective_bw;

        // We are not able to calculate correct network bandwidth at a given time
        // if the should_compress was calculated wrongs (compress not help but we say it help),
        // the whole transfer will be slowed down instead of being faster.
        // it is worse than not compressing.
        let imo_threshold = 0.92;
        (t_comp + t_send_compressed) < (t_send_raw * imo_threshold)
    }
}
