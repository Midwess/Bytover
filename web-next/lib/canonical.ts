/**
 * Get the canonical URL for a given path
 * @param path - The path relative to the root (e.g., '/transfer', '/oauth')
 * @returns The full canonical URL
 */
export function getCanonicalUrl(path: string = ''): string {
    // Get base URL from environment variable or detect from request
    const baseUrl = process.env.NEXT_PUBLIC_BASE_URL || 
                   (typeof window !== 'undefined' ? window.location.origin : 'https://bytover.com');
    
    // Ensure path starts with /
    const normalizedPath = path.startsWith('/') ? path : `/${path}`;
    
    // Remove trailing slash except for root
    const cleanPath = normalizedPath === '/' ? '/' : normalizedPath.replace(/\/$/, '');
    
    return `${baseUrl}${cleanPath}`;
}

/**
 * Generate metadata with canonical URL for Next.js pages
 * @param path - The path relative to the root
 * @param additionalMetadata - Additional metadata to merge
 * @returns Metadata object with canonical URL
 */
export function generateMetadataWithCanonical(
    path: string = '',
    additionalMetadata?: {
        title?: string;
        description?: string;
        [key: string]: unknown;
    }
) {
    const canonicalUrl = getCanonicalUrl(path);
    
    return {
        ...additionalMetadata,
        alternates: {
            canonical: canonicalUrl,
        },
    };
}

