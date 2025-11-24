'use client'
import {GitHubStarsButton} from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantAuthenticate,
    AuthenticationEventVariantSignOut,
} from 'shared_types/types/shared_types'
import {Button} from "@/components/ui/button.tsx";
import {
    DropdownMenu,
    DropdownMenuContent,
    DropdownMenuItem,
    DropdownMenuTrigger,
} from "@/components/animate-ui/radix/dropdown-menu";
import core from '@/wasm/wasm_core';

export default function Header() {
    const authState = core.useAuthenticationState();
    const isSignedIn = !!authState?.user;

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
        <div 
            className={`fixed backdrop-blur-lg bg-blackBase/50 top-0 left-0 right-0 z-100 flex justify-between items-center w-full py-3 md:py-6 px-3 md:px-4 transition-all duration-300`}>
            <div className="container mx-auto flex justify-between items-center w-full">
            <div className="flex flex-row gap-2 items-center">
                <Image
                   width={35}
                   height={35}
                   src="logo.svg"
                   alt="Logo"
                   className="rounded-lg aspect-square w-8 h-8 md:w-[45px] md:h-[45px]"
                />
            </div>

            <div className="hidden md:flex absolute left-1/2 transform -translate-x-1/2">
                <div className="flex flex-row gap-3 md:gap-5 rounded-xl px-4 md:px-8 py-2">
                    {[
                        { label: "Transfer", href: "#transfer" },
                        { label: "Pricing", href: "#pricing" },
                        { label: "Features", href: "#features" }
                    ].map((item) => (
                        <a
                            key={item.label}
                            href={item.href}
                            onClick={(e) => {
                                e.preventDefault();
                                scrollToSection(item.href);
                            }}
                            className="nav-link text-primaryText/80 text-sm md:text-base"
                        >
                            <h2 className="text-sm md:text-base">{item.label}</h2>
                        </a>
                    ))}
                </div>

            </div>
                <div className="flex flex-row gap-1.5 md:gap-2 font-bold text-primaryText items-center">
                    <GitHubStarsButton className={"hidden sm:flex under-development bg-muted-foreground/10 border h-8 md:h-10 text-foreground text-xs md:text-sm"} username="Dev-log" repo="animate-ui"/>
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
                                        scrollToSection('#transfer');
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
    )
}
