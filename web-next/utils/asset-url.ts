/**
 * Get the full asset URL with CDN prefix if configured
 * @param path - Asset path starting with /
 * @returns Full asset URL with CDN prefix or original path
 */
export const getAssetUrl = (path: string): string => {
    const prefix = process.env.NEXT_PUBLIC_S3_CDN_PREFIX;
    const version = process.env.NEXT_PUBLIC_VERSION || process.env.NEXT_PUBLIC_RAILWAY_GIT_COMMIT_SHA;

    if (prefix && version) {
        // Ensure path starts with /
        const normalizedPath = path.startsWith('/') ? path : `/${path}`;
        return `${prefix}/commit-${version}${normalizedPath}`;
    }

    return path;
};
