export function formatFileSize(model: {
  size_gb: number;
  size_mb: number;
  size_kb: number;
  size_bytes: bigint;
}): string {
  if (model.size_gb > 0) {
    return `${model.size_gb} GB`;
  }
  if (model.size_mb > 0) {
    return `${model.size_mb} MB`;
  }
  if (model.size_kb > 0) {
    return `${model.size_kb} KB`;
  }
  return `${model.size_bytes} Bytes`;
}
