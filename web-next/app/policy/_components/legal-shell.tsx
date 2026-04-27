import type { ReactNode } from 'react';
import Header from '@/components/web/header';
import Footer from '@/components/web/footer';
import { formatEffectiveDate } from './effective-date';

export type TocEntry = {
    id: string;
    label: string;
};

export type LegalShellProps = {
    title: string;
    effectiveDate: string;
    intro?: ReactNode;
    toc: TocEntry[];
    children: ReactNode;
};

export function LegalShell({ title, effectiveDate, intro, toc, children }: LegalShellProps) {
    return (
        <div className="min-h-screen bg-black text-primaryText flex flex-col">
            <Header />
            <main className="flex-1 py-16 px-4">
                <div className="container mx-auto max-w-5xl">
                    <header className="mb-10">
                        <h1 className="text-4xl font-bold mb-3">{title}</h1>
                        <p className="text-sm uppercase tracking-[0.2em] text-zinc-500">
                            Effective: {formatEffectiveDate(effectiveDate)}
                        </p>
                        {intro ? (
                            <div className="mt-6 text-muted-foreground leading-relaxed max-w-3xl">
                                {intro}
                            </div>
                        ) : null}
                    </header>

                    <div className="grid grid-cols-1 md:grid-cols-[220px_1fr] gap-10">
                        <aside className="hidden md:block">
                            <nav
                                aria-label="Table of contents"
                                className="sticky top-24 border-l border-zinc-800 pl-4"
                            >
                                <p className="text-xs uppercase tracking-[0.2em] text-zinc-500 mb-3">
                                    Contents
                                </p>
                                <ul className="flex flex-col gap-2">
                                    {toc.map((entry) => (
                                        <li key={entry.id}>
                                            <a
                                                href={`#${entry.id}`}
                                                className="text-sm text-zinc-400 hover:text-white transition-colors"
                                            >
                                                {entry.label}
                                            </a>
                                        </li>
                                    ))}
                                </ul>
                            </nav>
                        </aside>

                        <div>
                            <details className="md:hidden mb-8 border border-zinc-800 rounded-lg">
                                <summary className="cursor-pointer px-4 py-3 text-sm uppercase tracking-[0.2em] text-zinc-400">
                                    Contents
                                </summary>
                                <ul className="flex flex-col gap-2 px-4 pb-4">
                                    {toc.map((entry) => (
                                        <li key={entry.id}>
                                            <a
                                                href={`#${entry.id}`}
                                                className="text-sm text-zinc-400 hover:text-white transition-colors"
                                            >
                                                {entry.label}
                                            </a>
                                        </li>
                                    ))}
                                </ul>
                            </details>

                            <article className="prose prose-invert prose-lg max-w-none prose-headings:text-primaryText prose-strong:text-primaryText">
                                {children}
                            </article>
                        </div>
                    </div>
                </div>
            </main>
            <Footer />
        </div>
    );
}
