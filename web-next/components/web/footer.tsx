'use client'
import Link from "next/link";
import {cn} from "@/lib/utils.ts";
import { getAssetUrl } from '@/utils/asset-url';

export default function Footer(props: {className?: string | undefined, isFullWidth?: boolean, theme?: 'light' | 'dark'}) {
    const isLight = props.theme === 'light';
    
    const scrollToSection = (href: string) => {
        const element = document.querySelector(href);
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };

    return (
        <footer className={cn(
            "w-full border-t",
            isLight ? "bg-white border-zinc-200" : "bg-black border-white/5",
            props.className
        )}>
            <div className={cn("mx-auto px-4 sm:px-6 py-12 md:py-20", props.isFullWidth ? "container" : "container")}>
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
                            <span className={cn(
                                "font-bold text-lg tracking-tight",
                                isLight ? "text-zinc-900" : "text-white"
                            )}>Bytover</span>
                        </Link>
                        <p className={cn(
                            "text-sm leading-relaxed max-w-[200px]",
                            isLight ? "text-zinc-500" : "text-zinc-500"
                        )}>
                            High-performance file transfer for modern teams.
                        </p>
                    </div>

                    {/* Links Sections */}
                    <div className="grid grid-cols-2 md:grid-cols-3 col-span-1 md:col-span-3 gap-8">
                        <div className="flex flex-col gap-4">
                            <h4 className={cn(
                                "text-sm font-semibold tracking-tight",
                                isLight ? "text-zinc-900" : "text-white"
                            )}>Product</h4>
                            <ul className="flex flex-col gap-3">
                                <li><a href="#intro" onClick={(e) => { e.preventDefault(); scrollToSection('#intro'); }} className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Overview</a></li>
                                <li><a href="#features" onClick={(e) => { e.preventDefault(); scrollToSection('#features'); }} className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Features</a></li>
                                <li><a href="#pricing" onClick={(e) => { e.preventDefault(); scrollToSection('#pricing'); }} className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Pricing</a></li>
                                <li><Link href="/transfer" className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Web App</Link></li>
                            </ul>
                        </div>
                        <div className="flex flex-col gap-4">
                            <h4 className={cn(
                                "text-sm font-semibold tracking-tight",
                                isLight ? "text-zinc-900" : "text-white"
                            )}>Legal</h4>
                            <ul className="flex flex-col gap-3">
                                <li><Link href="/policy/privacy" className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Privacy</Link></li>
                                <li><Link href="/policy/terms" className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>Terms</Link></li>
                                <li><Link href="/policy/eula" className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>EULA</Link></li>
                            </ul>
                        </div>
                        <div className="flex flex-col gap-4">
                            <h4 className={cn(
                                "text-sm font-semibold tracking-tight",
                                isLight ? "text-zinc-900" : "text-white"
                            )}>Connect</h4>
                            <ul className="flex flex-col gap-3">
                                <li><Link href="https://github.com/Midwess/Bytover" target="_blank" className={cn("hover:text-zinc-900 transition-colors text-sm", isLight ? "text-zinc-500" : "text-zinc-500 hover:text-white")}>GitHub</Link></li>

                            </ul>
                        </div>
                    </div>
                </div>

                <div className={cn(
                    "mt-20 pt-8 border-t flex flex-col md:flex-row justify-between items-center gap-4",
                    isLight ? "border-zinc-100" : "border-white/5"
                )}>
                    <p className={cn(
                        "text-xs font-bold uppercase tracking-[0.2em]",
                        isLight ? "text-zinc-400" : "text-zinc-600"
                    )}>
                        © {new Date().getFullYear()} Bytover. All rights reserved.
                    </p>
                    <div className="flex items-center gap-6">
                         <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-[0_0_8px_rgba(16,185,129,0.5)]" />
                         <span className="text-xs font-bold uppercase tracking-[0.2em] text-emerald-500">All systems operational</span>
                    </div>
                </div>
            </div>
        </footer>
    );
}

