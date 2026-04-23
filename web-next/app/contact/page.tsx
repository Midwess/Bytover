import type { Metadata } from 'next';
import { Suspense } from 'react';
import { getCanonicalUrl } from '@/lib/canonical';
import { getAssetUrl } from '@/utils/asset-url';
import Header from '@/components/web/header';
import Footer from '@/components/web/footer';
import { ContactContent } from '@/app/contact/contact-content';

export const metadata: Metadata = {
    title: 'Contact - Bytover',
    description: "Get in touch with the Bytover team. Share feedback, ask questions, or tell us what you'd like to see next.",
    openGraph: {
        title: 'Contact - Bytover',
        description: "Get in touch with the Bytover team. Share feedback, ask questions, or tell us what you'd like to see next.",
        url: getCanonicalUrl('/contact'),
        siteName: 'Bytover',
        locale: 'en_US',
        type: 'website',
    },
    twitter: {
        card: 'summary_large_image',
        title: 'Contact - Bytover',
        description: "Get in touch with the Bytover team. Share feedback, ask questions, or tell us what you'd like to see next.",
    },
    alternates: {
        canonical: getCanonicalUrl('/contact'),
    },
};

export default function ContactPage() {
    return (
        <div className="min-h-screen bg-black text-primaryText flex flex-col">
            <Suspense fallback={null}>
                <Header className="px-3" />
            </Suspense>

            <main className="flex-1 relative overflow-hidden flex items-center justify-center px-4 py-20 md:py-32">
                <div className="absolute inset-0 z-0">
                    <img
                        src={getAssetUrl('/background6.jpg')}
                        alt=""
                        className="w-full h-full object-cover opacity-20"
                    />
                    <div className="absolute inset-0 bg-gradient-to-b from-black/60 via-[#080410]/70 to-black" />
                    <div className="absolute inset-0 opacity-[0.15] mix-blend-overlay pointer-events-none" style={{ backgroundImage: `url("data:image/svg+xml,%3Csvg viewBox='0 0 200 200' xmlns='http://www.w3.org/2000/svg'%3E%3Cfilter id='noiseFilter'%3E%3CfeTurbulence type='fractalNoise' baseFrequency='0.65' numOctaves='3' stitchTiles='stitch'/%3E%3C/svg%3E#noiseFilter")` }} />
                    <div className="absolute inset-0 pointer-events-none overflow-hidden mix-blend-overlay hidden dark:block bg-purple-500/5 backdrop-blur-[2px]" />
                </div>

                <div className="relative z-10 w-full">
                    <ContactContent />
                </div>
            </main>

            <Footer className="bg-black border-t border-white/5" />
        </div>
    );
}
