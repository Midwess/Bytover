import { MetadataRoute } from 'next';
import { getCanonicalUrl } from '@/lib/canonical';

export default function sitemap(): MetadataRoute.Sitemap {
    return [
        {
            url: getCanonicalUrl('/'),
            lastModified: new Date(),
            changeFrequency: 'weekly',
            priority: 1.0,
        },
        {
            url: getCanonicalUrl('/transfer'),
            lastModified: new Date(),
            changeFrequency: 'weekly',
            priority: 1.0,
        },
        {
            url: getCanonicalUrl('/contact'),
            lastModified: new Date(),
            changeFrequency: 'monthly',
            priority: 0.7,
        },
    ];
}
