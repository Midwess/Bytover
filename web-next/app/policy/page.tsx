import type { Metadata } from 'next';
import Link from 'next/link';
import { getCanonicalUrl } from '@/lib/canonical';
import Header from '@/components/web/header';
import Footer from '@/components/web/footer';
import {
    PRIVACY_EFFECTIVE_DATE,
    TERMS_EFFECTIVE_DATE,
    EULA_EFFECTIVE_DATE,
    formatEffectiveDate,
} from './_components/effective-date';

export const metadata: Metadata = {
    title: 'Legal — Bytover',
    description:
        'Bytover privacy policy, terms of service, and end-user license agreement.',
    alternates: {
        canonical: getCanonicalUrl('/policy'),
    },
};

type LegalDocument = {
    href: string;
    title: string;
    description: string;
    effectiveDate: string;
};

const DOCUMENTS: LegalDocument[] = [
    {
        href: '/policy/privacy',
        title: 'Privacy Policy',
        description:
            'What data Bytover collects, how it is used, and your rights. We do not share your data with third parties for analysis.',
        effectiveDate: PRIVACY_EFFECTIVE_DATE,
    },
    {
        href: '/policy/terms',
        title: 'Terms of Service',
        description:
            'The agreement governing your use of Bytover, including acceptable use and DMCA.',
        effectiveDate: TERMS_EFFECTIVE_DATE,
    },
    {
        href: '/policy/eula',
        title: 'End User License Agreement',
        description:
            'License terms for the Bytover client software and details of the one-time Paid Plan purchase.',
        effectiveDate: EULA_EFFECTIVE_DATE,
    },
];

export default function PolicyIndexPage() {
    return (
        <div className="min-h-screen bg-black text-primaryText flex flex-col">
            <Header />
            <main className="flex-1 py-16 px-4">
                <div className="container mx-auto max-w-4xl">
                    <header className="mb-12">
                        <h1 className="text-4xl font-bold mb-4">Legal</h1>
                        <p className="text-muted-foreground leading-relaxed max-w-2xl">
                            Bytover is a file-transfer product designed so your files travel directly
                            between devices whenever possible. We do not share your data with any third
                            party for analysis, advertising, or profiling. The documents below describe
                            the details.
                        </p>
                    </header>

                    <ul className="grid grid-cols-1 md:grid-cols-3 gap-6">
                        {DOCUMENTS.map((doc) => (
                            <li key={doc.href}>
                                <Link
                                    href={doc.href}
                                    className="group block h-full p-6 border border-zinc-800 rounded-lg hover:border-zinc-600 transition-colors"
                                >
                                    <h2 className="text-xl font-semibold mb-3 group-hover:text-white">
                                        {doc.title}
                                    </h2>
                                    <p className="text-sm text-muted-foreground leading-relaxed mb-6">
                                        {doc.description}
                                    </p>
                                    <p className="text-xs uppercase tracking-[0.2em] text-zinc-500">
                                        Effective {formatEffectiveDate(doc.effectiveDate)}
                                    </p>
                                </Link>
                            </li>
                        ))}
                    </ul>

                    <p className="mt-12 text-sm text-muted-foreground">
                        Questions? Email{' '}
                        <a
                            href="mailto:team@bytover.com"
                            className="underline hover:text-white"
                        >
                            team@bytover.com
                        </a>
                        .
                    </p>
                </div>
            </main>
            <Footer />
        </div>
    );
}
