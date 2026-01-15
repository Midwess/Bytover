'use client'
import Link from "next/link";
import {cn} from "@/lib/utils.ts";
import { getAssetUrl } from '@/utils/asset-url';

export default function Footer(props: {className?: string | undefined} = { className: '' }) {
    const scrollToSection = (href: string) => {
        const element = document.querySelector(href);
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };

    return (
        <footer className={cn("w-full bg-black border-t border-zinc-800", props.className)}>
            <div className="container mx-auto px-4 sm:px-6 py-8 sm:py-12 md:py-16">
                {/* Main Footer Content */}
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-8 md:gap-12">
                    {/* Brand Section - Full width on mobile, spans 2 cols on tablet+ */}
                    <div className="flex flex-col gap-4 sm:col-span-2 lg:col-span-1">
                        <div className="flex items-center gap-3">
                            <img
                                width={40}
                                height={40}
                                src={getAssetUrl('/logo.png')}
                                alt="Bytover Logo"
                                className="rounded-lg aspect-square w-10 h-10"
                            />
                            <h2 className="text-xl font-bold text-primaryText">Bytover</h2>
                        </div>
                        <h2 className="text-muted-foreground text-sm max-w-xs">
                            A seamless file transfer that you can trust.
                        </h2>
                    </div>

                    {/* Sections Links */}
                    <div className="flex flex-col gap-4">
                        <h3 className="font-semibold text-primaryText text-base">Sections</h3>
                        <ul className="flex flex-col gap-2.5">
                            <li>
                                <a
                                    href="#intro"
                                    onClick={(e) => {
                                        e.preventDefault();
                                        scrollToSection('#intro');
                                    }}
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Home
                                </a>
                            </li>
                            <li>
                                <a
                                    href="#more-features"
                                    onClick={(e) => {
                                        e.preventDefault();
                                        scrollToSection('#more-features');
                                    }}
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Features
                                </a>
                            </li>
                            <li>
                                <a
                                    href="#pricing"
                                    onClick={(e) => {
                                        e.preventDefault();
                                        scrollToSection('#pricing');
                                    }}
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Pricing
                                </a>
                            </li>
                            <li>
                                <a
                                    href="#waitlist"
                                    onClick={(e) => {
                                        e.preventDefault();
                                        scrollToSection('#waitlist');
                                    }}
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Join Waitlist
                                </a>
                            </li>
                        </ul>
                    </div>

                    {/* Support Links */}
                    <div className="flex flex-col gap-4">
                        <h3 className="font-semibold text-primaryText text-base">Support</h3>
                        <ul className="flex flex-col gap-2.5">
                            <li>
                                <Link
                                    href="#"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    FAQ
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="#"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Contact
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="/policy"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Privacy Policy
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="#"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Terms of Service
                                </Link>
                            </li>
                        </ul>
                    </div>

                    {/* Resources Links */}
                    <div className="flex flex-col gap-4">
                        <h3 className="font-semibold text-primaryText text-base">Resources</h3>
                        <ul className="flex flex-col gap-2.5">
                            <li>
                                <Link
                                    href="/transfer"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Transfer Files
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="#"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Documentation
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="#"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    Blog
                                </Link>
                            </li>
                            <li>
                                <Link
                                    href="https://github.com/dev-logs/bit-bridge"
                                    target="_blank"
                                    rel="noopener noreferrer"
                                    className="text-muted-foreground hover:text-primaryText transition-colors text-sm inline-block"
                                >
                                    GitHub
                                </Link>
                            </li>
                        </ul>
                    </div>
                </div>

                {/* Divider */}
                <div className="border-t border-zinc-800 mt-8 sm:mt-12 pt-6 sm:pt-8">
                    {/* Copyright and Social */}
                    <div className="flex flex-col sm:flex-row justify-between items-center gap-4">
                        <p className="text-muted-foreground text-xs sm:text-sm text-center sm:text-left">
                            © {new Date().getFullYear()} Bytover. All rights reserved.
                        </p>

                        {/* Social Links */}
                        <div className="flex items-center gap-4 under-development">
                            <Link
                                href="https://github.com/dev-logs/bit-bridge"
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-muted-foreground hover:text-primaryText transition-colors"
                                aria-label="GitHub"
                            >
                                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                    <path fillRule="evenodd" d="M12 2C6.477 2 2 6.484 2 12.017c0 4.425 2.865 8.18 6.839 9.504.5.092.682-.217.682-.483 0-.237-.008-.868-.013-1.703-2.782.605-3.369-1.343-3.369-1.343-.454-1.158-1.11-1.466-1.11-1.466-.908-.62.069-.608.069-.608 1.003.07 1.531 1.032 1.531 1.032.892 1.53 2.341 1.088 2.91.832.092-.647.35-1.088.636-1.338-2.22-.253-4.555-1.113-4.555-4.951 0-1.093.39-1.988 1.029-2.688-.103-.253-.446-1.272.098-2.65 0 0 .84-.27 2.75 1.026A9.564 9.564 0 0112 6.844c.85.004 1.705.115 2.504.337 1.909-1.296 2.747-1.027 2.747-1.027.546 1.379.202 2.398.1 2.651.64.7 1.028 1.595 1.028 2.688 0 3.848-2.339 4.695-4.566 4.943.359.309.678.92.678 1.855 0 1.338-.012 2.419-.012 2.747 0 .268.18.58.688.482A10.019 10.019 0 0022 12.017C22 6.484 17.522 2 12 2z" clipRule="evenodd" />
                                </svg>
                            </Link>
                            <Link
                                href="#"
                                className="text-muted-foreground hover:text-primaryText transition-colors"
                                aria-label="Twitter"
                            >
                                <svg className="w-5 h-5" fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
                                    <path d="M18.244 2.25h3.308l-7.227 8.26 8.502 11.24H16.17l-5.214-6.817L4.99 21.75H1.68l7.73-8.835L1.254 2.25H8.08l4.713 6.231zm-1.161 17.52h1.833L7.084 4.126H5.117z" />
                                </svg>
                            </Link>
                        </div>
                    </div>
                </div>
            </div>
        </footer>
    );
}

