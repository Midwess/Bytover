'use client'
import React from 'react';
import { GitHubStarsButton } from '@/components/animate-ui/buttons/github-stars'
import { getAssetUrl } from '@/utils/asset-url';
import { Loader2 } from "lucide-react";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantAuthenticate,
    AuthenticationEventVariantSignOut,
} from 'shared_types/types/shared_types'
import { Button } from "@/components/ui/button.tsx";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/animate-ui/radix/dropdown-menu";
import core from '@/wasm/wasm_core';
import Link from 'next/link';
import { cn } from '@/lib/utils';

export default function Header({ className }: { className?: string }) {
    const coreReady = core.useCoreReady();
    const authState = core.useAuthenticationState()
    const isSignedIn = coreReady && !!authState?.user;
    const [isScrolled, setIsScrolled] = React.useState(false);
    const sentinelRef = React.useRef<HTMLDivElement>(null);

    React.useEffect(() => {
        const observer = new IntersectionObserver(
            ([entry]) => {
                setIsScrolled(!entry.isIntersecting);
            },
            {
                threshold: 0,
                rootMargin: '0px 0px 0px 0px',
            }
        );

        if (sentinelRef.current) {
            observer.observe(sentinelRef.current);
        }

        return () => {
            if (sentinelRef.current) {
                observer.unobserve(sentinelRef.current);
            }
        };
    }, []);

    const scrollToSection = (href: string) => {
        const element = document.querySelector(href);
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };

    const handleSignOut = () => {
        core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantSignOut()));
        setTimeout(() => {
            // window.location.reload();
        }, 1000);
    };

    return (
        <>
            {/* Invisible sentinel element to detect scroll */}
            <div ref={sentinelRef} className="absolute top-0 left-0 w-full h-px pointer-events-none" />

            <div
                className={`fixed top-0 left-0 right-0 z-100 flex justify-center w-full transition-all duration-500 pt-4 md:pt-6 px-4 md:px-6`}>
                <div className={cn(
                    "flex justify-between items-center w-full container px-4 md:px-6 h-16 md:h-20 transition-all duration-500 rounded-xl md:rounded-2xl",
                    isScrolled 
                        ? "bg-black/60 backdrop-blur-xl border border-white/10 shadow-[0_8px_32px_-8px_rgba(0,0,0,0.5)]" 
                        : "bg-transparent border border-transparent",
                    className
                )}>
                    <div className="flex items-center gap-8">
                        <Link href="/" className="flex items-center group">
                            <img
                                width={32}
                                height={32}
                                src={getAssetUrl('/logo.png')}
                                alt="Logo"
                                className="rounded-lg aspect-square w-8 h-8 group-hover:opacity-80 transition-opacity"
                            />
                            <span className="ml-2.5 font-bold text-lg tracking-tight text-white hidden sm:block">Bytover</span>
                        </Link>

                        <nav className="hidden md:flex items-center gap-6">
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
                                    className="text-sm font-medium text-zinc-400 hover:text-white transition-colors"
                                >
                                    {item.label}
                                </Link>
                            ))}
                        </nav>
                    </div>

                    <div className="flex items-center gap-3">
                        <GitHubStarsButton className="hidden lg:flex bg-zinc-900 border-zinc-800 text-zinc-400 text-xs h-9" username="Dev-log" repo="animate-ui" />
                        
                        {isSignedIn ? (
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="outline"
                                        className="h-9 bg-zinc-900 border-zinc-800 text-zinc-300 hover:bg-zinc-800 hover:text-white text-sm px-4"
                                    >
                                        {authState?.user?.email?.split('@')[0] || 'Account'}
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="!z-[200] bg-zinc-950 border-zinc-800 text-zinc-300">
                                    <DropdownMenuItem
                                        onClick={(e) => {
                                            e.preventDefault();
                                            window.location.href = '/transfer';
                                        }}
                                        className="focus:bg-zinc-900 focus:text-white"
                                    >
                                        Transfer
                                    </DropdownMenuItem>
                                    <DropdownMenuItem
                                        className="text-red-400 focus:bg-red-950/30 focus:text-red-400"
                                        onClick={handleSignOut}
                                    >
                                        Sign out
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        ) : (
                            <Button
                                variant="default"
                                disabled={!coreReady}
                                className="h-9 bg-white text-black hover:bg-zinc-200 text-sm font-semibold px-5 transition-all"
                                onClick={() => core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantAuthenticate()))}
                            >
                                {!coreReady ? (
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                ) : (
                                    "Sign in"
                                )}
                            </Button>
                        )}
                    </div>
                </div>
            </div>
        </>
    )
}
