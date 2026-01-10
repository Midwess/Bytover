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
        "description": "File sharing with instant direct transfers, public sharing with password protection, and email delivery."
    };

    const homepageSchema = {
        "@context": "https://schema.org",
        "@type": "WebPage",
        "name": "Bytover Home",
        "url": baseUrl,
        "description": "File sharing with instant direct transfers, public sharing with password protection, and email delivery.",
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
        "description": "Secure file sharing platform with instant direct transfers, public sharing with password protection, and email delivery.",
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

