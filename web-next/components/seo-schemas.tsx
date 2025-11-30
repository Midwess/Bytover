import { getCanonicalUrl } from '@/lib/canonical';
import Script from 'next/script';

export function SEOSchemas() {
    const baseUrl = getCanonicalUrl('');

    const organizationSchema = {
        "@context": "https://schema.org",
        "@type": "Organization",
        "name": "Midwess",
        "url": "https://midwess.com"
    };

    const websiteSchema = {
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": "Bytover",
        "url": baseUrl,
        "description": "File sharing, including Peer to peer, Nearby transfer, creating Public file transfer with Password protected, or sending To email."
    };

    const homepageSchema = {
        "@context": "https://schema.org",
        "@type": "WebPage",
        "name": "Bytover Home",
        "url": baseUrl,
        "description": "File sharing, including Peer to peer, Nearby transfer, creating Public file transfer with Password protected, or sending To email.",
        "publisher": {
            "@type": "Organization",
            "name": "Midwess",
            "url": "https://midwess.com"
        }
    };

    const serviceSchema = {
        "@context": "https://schema.org",
        "@type": "Service",
        "name": "Bytover File Sharing",
        "url": baseUrl,
        "description": "Secure file sharing platform, including Peer to peer, Nearby transfer, Public file transfer with Password protected, and sending To email.",
        "provider": {
            "@type": "Organization",
            "name": "Midwess",
            "url": "https://midwess.com"
        }
    };

    return (
        <>
            <Script async src="https://www.googletagmanager.com/gtag/js?id=G-2CC70F53Q7"></Script>
            <Script id="ga-init" strategy="afterInteractive">
                {`
          window.dataLayer = window.dataLayer || [];
          function gtag(){dataLayer.push(arguments);}
          gtag('js', new Date());
  gtag('config', 'G-2CC70F53Q7');
        `}
            </Script>
            <Script
                id="schema-organization"
                type="application/ld+json"
                dangerouslySetInnerHTML={{ __html: JSON.stringify(organizationSchema) }}
            />
            <Script
                id="schema-website"
                type="application/ld+json"
                dangerouslySetInnerHTML={{ __html: JSON.stringify(websiteSchema) }}
            />
            <Script
                id="schema-homepage"
                type="application/ld+json"
                dangerouslySetInnerHTML={{ __html: JSON.stringify(homepageSchema) }}
            />
            <Script
                id="schema-service"
                type="application/ld+json"
                dangerouslySetInnerHTML={{ __html: JSON.stringify(serviceSchema) }}
            />
        </>
    );
}

