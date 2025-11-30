import { MetadataRoute } from 'next';
import { getCanonicalUrl } from '@/lib/canonical';

export default function robots(): MetadataRoute.Robots {
    const baseUrl = getCanonicalUrl('');
    
    return {
        rules: [
            {
                userAgent: '*',
                allow: '/',
                disallow: ['/cgi-bin/', '/oauth'],
            },
        ],
        sitemap: `${baseUrl}/sitemap.xml`,
    };
}
