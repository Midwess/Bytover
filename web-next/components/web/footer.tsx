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
        <footer className={cn("w-full bg-black border-t border-white/5", props.className)}>
            <div className="container mx-auto px-4 sm:px-6 py-12 md:py-20">
                <div className="grid grid-cols-1 md:grid-cols-4 gap-12">
                    {/* Brand Section */}
                    <div className="col-span-1 md:col-span-1 flex flex-col gap-6">
                        <Link href="/" className="flex items-center gap-3 group">
                            <img
                                width={32}
                                height={32}
                                src={getAssetUrl('/logo.png')}
                                alt="Logo"
                                className="rounded-lg aspect-square w-8 h-8 group-hover:opacity-80 transition-opacity"
                            />
                            <span className="font-bold text-lg tracking-tight text-white">Bytover</span>
                        </Link>
                        <p className="text-zinc-500 text-sm leading-relaxed max-w-[200px]">
                            High-performance file transfer for modern teams.
                        </p>
                    </div>

                    {/* Links Sections */}
                    <div className="grid grid-cols-2 md:grid-cols-3 col-span-1 md:col-span-3 gap-8">
                        <div className="flex flex-col gap-4">
                            <h4 className="text-white text-sm font-semibold tracking-tight">Product</h4>
                            <ul className="flex flex-col gap-3">
                                <li><a href="#intro" onClick={(e) => { e.preventDefault(); scrollToSection('#intro'); }} className="text-zinc-500 hover:text-white transition-colors text-sm">Overview</a></li>
                                <li><a href="#features" onClick={(e) => { e.preventDefault(); scrollToSection('#features'); }} className="text-zinc-500 hover:text-white transition-colors text-sm">Features</a></li>
                                <li><a href="#pricing" onClick={(e) => { e.preventDefault(); scrollToSection('#pricing'); }} className="text-zinc-500 hover:text-white transition-colors text-sm">Pricing</a></li>
                                <li><Link href="/transfer" className="text-zinc-500 hover:text-white transition-colors text-sm">Web App</Link></li>
                            </ul>
                        </div>
                        <div className="flex flex-col gap-4">
                            <h4 className="text-white text-sm font-semibold tracking-tight">Legal</h4>
                            <ul className="flex flex-col gap-3">
                                <li><Link href="/policy" className="text-zinc-500 hover:text-white transition-colors text-sm">Privacy</Link></li>
                                <li><Link href="#" className="text-zinc-500 hover:text-white transition-colors text-sm">Terms</Link></li>
                                <li><Link href="#" className="text-zinc-500 hover:text-white transition-colors text-sm">Cookies</Link></li>
                            </ul>
                        </div>
                        <div className="flex flex-col gap-4">
                            <h4 className="text-white text-sm font-semibold tracking-tight">Connect</h4>
                            <ul className="flex flex-col gap-3">
                                <li><Link href="https://github.com/dev-logs/bit-bridge" target="_blank" className="text-zinc-500 hover:text-white transition-colors text-sm">GitHub</Link></li>
                                <li><Link href="#" className="text-zinc-500 hover:text-white transition-colors text-sm">Twitter</Link></li>
                                <li><Link href="#" className="text-zinc-500 hover:text-white transition-colors text-sm">Discord</Link></li>
                            </ul>
                        </div>
                    </div>
                </div>

                <div className="mt-20 pt-8 border-t border-white/5 flex flex-col md:flex-row justify-between items-center gap-4">
                    <p className="text-zinc-600 text-[10px] font-bold uppercase tracking-[0.2em]">
                        © {new Date().getFullYear()} Bytover. All rights reserved.
                    </p>
                    <div className="flex items-center gap-6">
                         <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                         <span className="text-[10px] font-bold uppercase tracking-[0.2em] text-emerald-500">All systems operational</span>
                    </div>
                </div>
            </div>
        </footer>
    );
}

