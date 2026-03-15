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

export default function Header({ className, isFullWidth, theme = 'dark' }: { className?: string, isFullWidth?: boolean, theme?: 'light' | 'dark' }) {
    const coreReady = core.useCoreReady();
    const authState = core.useAuthenticationState()
    const isSignedIn = coreReady && !!authState?.user;
    const [isScrolled, setIsScrolled] = React.useState(false);
    const sentinelRef = React.useRef<HTMLDivElement>(null);

    const isLight = theme === 'light';

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
                className={cn(
                    "fixed top-0 left-0 right-0 z-[100] w-full transition-all duration-500 border-b",
                    isScrolled 
                        ? (isLight 
                            ? "bg-white border-zinc-200 shadow-none" 
                            : "bg-black border-white/10 shadow-none")
                        : "bg-transparent border-transparent",
                    className
                )}>
                <div className="flex justify-between items-center w-full px-4 md:px-8 h-16 md:h-20">
                    <div className="flex items-center gap-8">
                        <Link href="/" className="flex items-center group">
                            <img
                                width={32}
                                height={32}
                                src={getAssetUrl('/logo.png')}
                                alt="Logo"
                                className="rounded-lg aspect-square w-8 h-8 group-hover:opacity-80 transition-opacity"
                            />
                            <span className={cn(
                                "ml-2.5 font-bold text-lg tracking-tight hidden sm:block",
                                isLight ? "text-zinc-900" : "text-white"
                            )}>Bytover</span>
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
                                    className={cn(
                                        "text-sm font-medium transition-colors",
                                        isLight ? "text-zinc-500 hover:text-zinc-900" : "text-zinc-400 hover:text-white"
                                    )}
                                >
                                    {item.label}
                                </Link>
                            ))}
                        </nav>
                    </div>

                    <div className="flex items-center gap-3">
                        <GitHubStarsButton className={cn(
                            "hidden lg:flex text-xs h-9",
                            isLight ? "bg-white border-zinc-200 text-zinc-600" : "bg-zinc-900 border-zinc-800 text-zinc-400"
                        )} username="Dev-log" repo="animate-ui" />
                        
                        {isSignedIn ? (
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="outline"
                                        className={cn(
                                            "h-9 text-sm px-4",
                                            isLight ? "bg-white border-zinc-200 text-zinc-700 hover:bg-zinc-50" : "bg-zinc-900 border-zinc-800 text-zinc-300 hover:bg-zinc-800 hover:text-white"
                                        )}
                                    >
                                        {authState?.user?.email?.split('@')[0] || 'Account'}
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className={cn(
                                    "!z-[200]",
                                    isLight ? "bg-white border-zinc-200 text-zinc-700" : "bg-zinc-950 border-zinc-800 text-zinc-300"
                                )}>
                                    <DropdownMenuItem
                                        onClick={(e) => {
                                            e.preventDefault();
                                            window.location.href = '/transfer';
                                        }}
                                        className={cn(isLight ? "focus:bg-zinc-50 focus:text-zinc-900" : "focus:bg-zinc-900 focus:text-white")}
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
                                className={cn(
                                    "h-9 text-sm font-semibold px-5 transition-all",
                                    isLight ? "bg-zinc-900 text-white hover:bg-black" : "bg-white text-black hover:bg-zinc-200"
                                )}
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
