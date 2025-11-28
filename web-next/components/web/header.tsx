'use client'
import React from 'react';
import { GitHubStarsButton } from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";
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
    const authState = core.useAuthenticationState();
    const isSignedIn = !!authState?.user;
    const [isScrolled, setIsScrolled] = React.useState(false);
    const sentinelRef = React.useRef<HTMLDivElement>(null);

    React.useEffect(() => {
        const observer = new IntersectionObserver(
            ([entry]) => {
                // When sentinel is not visible, we've scrolled down
                setIsScrolled(!entry.isIntersecting);
            },
            {
                threshold: 0,
                rootMargin: '0px 0px 0px 0px', // Trigger when 50px from top
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
            window.location.reload();
        }, 1000);
    };

    return (
        <>
            {/* Invisible sentinel element to detect scroll */}
            <div ref={sentinelRef} className="absolute top-0 left-0 w-full h-px pointer-events-none" />

            <div
                className={`fixed top-0 left-0 right-0 z-100 flex justify-between items-center w-full py-3 md:py-6 transition-all duration-300 ${isScrolled ? 'bg-black/90 backdrop-blur-md shadow-lg' : 'bg-transparent backdrop-blur-none'
                    }`}>
                <div className={cn("flex justify-between items-center w-full container px-3", className)}>
                    <div className="flex flex-row gap-2 items-center">
                        <Link href="/" className="flex items-center">
                            <Image
                                width={35}
                                height={35}
                                src="logo.svg"
                                alt="Logo"
                                className="rounded-lg aspect-square w-8 h-8 md:w-[45px] md:h-[45px]"
                            />
                        </Link>
                    </div>

                    <div className="hidden md:flex absolute left-1/2 transform -translate-x-1/2">
                        <div className={`flex flex-row gap-3 md:gap-5 rounded-xl px-4 md:px-8 py-2 transition-all duration-300 ${!isScrolled ? 'bg-black/20 backdrop-blur-sm border border-white/5' : ''
                            }`}>
                            {[
                                { label: "Transfer", href: "/transfer" },
                                { label: "Pricing", href: "#pricing" },
                                { label: "Features", href: "#features" }
                            ].map((item) => (
                                <Link
                                    key={item.label}
                                    href={item.href}
                                    onClick={(e) => {
                                        e.preventDefault();

                                        if (item.href.startsWith('#')) {
                                            // Always go to home first, then scroll
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
                                    className="nav-link text-primaryText/80 text-sm md:text-base hover:text-primaryText transition-colors"
                                >
                                    <h2 className="text-sm md:text-base">{item.label}</h2>
                                </Link>
                            ))}
                        </div>

                    </div>
                    <div className="flex flex-row gap-1.5 md:gap-2 font-bold text-primaryText items-center">
                        <GitHubStarsButton className={"hidden sm:flex under-development bg-muted-foreground/10 border h-8 md:h-10 text-foreground text-xs md:text-sm"} username="Dev-log" repo="animate-ui" />
                        {isSignedIn ? (
                            <DropdownMenu>
                                <DropdownMenuTrigger asChild>
                                    <Button
                                        variant="outline"
                                        className="h-8 md:h-10 bg-background/80 backdrop-blur-sm border-border/50 text-primaryText hover:bg-background/90 text-xs md:text-sm px-3 md:px-4"
                                    >
                                        {authState?.user?.email || 'Account'}
                                    </Button>
                                </DropdownMenuTrigger>
                                <DropdownMenuContent align="end" className="!z-[200]">
                                    <DropdownMenuItem
                                        onClick={(e) => {
                                            e.preventDefault();
                                            window.location.href = '/transfer';
                                        }}
                                    >
                                        Transfer
                                    </DropdownMenuItem>
                                    <DropdownMenuItem
                                        onClick={handleSignOut}
                                    >
                                        Sign out
                                    </DropdownMenuItem>
                                </DropdownMenuContent>
                            </DropdownMenu>
                        ) : (
                            <Button
                                variant="default"
                                className="h-8 md:h-10 bg-bluePrimary text-white text-xs md:text-sm px-3 md:px-4"
                                onClick={() => core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantAuthenticate()))}
                            >
                                Sign in
                            </Button>
                        )}
                    </div>
                </div>
            </div>
        </>
    )
}
