import React from 'react';
import { getAssetUrl } from '@/utils/asset-url';
import Link from 'next/link';

export default function StaticHeader({ className, isFullWidth }: { className?: string, isFullWidth?: boolean }) {
    const scrollToSection = (href: string) => {
        const element = document.querySelector(href);
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };

    return (
        <div className={`w-full h-fit border-white/5 ${className || ''}`}>
            <div className={`flex justify-between items-center w-full h-full px-4 md:px-6 ${isFullWidth ? 'max-w-[1360px] mx-auto' : 'container'}`}>
                <div className="flex items-center gap-2">
                    <Link href="/" className="flex items-center group">
                        <img
                            width={32}
                            height={32}
                            src={getAssetUrl('/logo.png')}
                            alt="Logo"
                            className="rounded-lg aspect-square w-8 h-8 group-hover:opacity-80 transition-opacity"
                        />
                        <span className="ml-2.5 font-bold text-lg tracking-tight hidden sm:block text-white">Bytover</span>
                    </Link>
                </div>

                <nav className="hidden md:flex items-center gap-6 absolute left-1/2 transform -translate-x-1/2">
                    {[
                        { label: "Transfer", href: "/transfer" },
                        { label: "Pricing", href: "#pricing" },
                        { label: "Features", href: "#more-features" }
                    ].map((item) => (
                        <Link
                            key={item.label}
                            href={item.href}
                            onClick={(e) => {
                                e.preventDefault();
                                if (item.href.startsWith('#')) {
                                    const target = `/${item.href}`;
                                    if (window.location.pathname !== '/') {
                                        window.location.href = target;
                                    } else {
                                        scrollToSection(item.href);
                                    }
                                } else {
                                    window.location.href = item.href;
                                }
                            }}
                            className="text-sm font-medium transition-colors text-foreground hover:text-white"
                        >
                            {item.label}
                        </Link>
                    ))}
                </nav>

                <div className="flex items-center gap-2">
                </div>
           </div>
        </div>
    )
}
