'use client'
import {GitHubStarsButton} from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantAuthenticate,
} from 'shared_types/types/shared_types'
import {Button} from "@/components/ui/button.tsx";
import { useEffect, useState } from "react";

export default function Header() {
    const [isVisible, setIsVisible] = useState(true);
    const [lastScrollY, setLastScrollY] = useState(0);
    const [isAtTop, setIsAtTop] = useState(true);

    useEffect(() => {
        const controlHeader = () => {
            const currentScrollY = window.scrollY;
            
            // Check if at top
            setIsAtTop(currentScrollY < 100);
            
            if (currentScrollY < 100) {
                setIsVisible(true);
            } else if (currentScrollY > lastScrollY) {
                // Scrolling down
                setIsVisible(false);
            } else {
                // Scrolling up
                setIsVisible(true);
            }
            
            setLastScrollY(currentScrollY);
        };

        window.addEventListener('scroll', controlHeader);
        return () => window.removeEventListener('scroll', controlHeader);
    }, [lastScrollY]);

    const onAuthenticate= () => {
        core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantAuthenticate()))
    }

    return (
        <div className={`fixed top-0 left-0 right-0 z-50 flex justify-between items-center w-full py-3 md:py-6 px-3 md:px-4 transition-all duration-300 ${isVisible ? 'translate-y-0' : '-translate-y-full'} ${isAtTop ? 'bg-transparent border-b border-transparent shadow-none' : 'backdrop-blur-xl bg-blackBase/90 border-b border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.37)]'}`}>
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
                                const element = document.querySelector(item.href);
                                if (element) {
                                    element.scrollIntoView({ behavior: 'smooth', block: 'start' });
                                }
                            }}
                            className="nav-link text-primaryText/80 text-sm md:text-base"
                        >
                            <h2 className="text-sm md:text-base">{item.label}</h2>
                        </a>
                    ))}
                </div>

                <style>{`
                    .nav-link {
                      position: relative;
                      text-decoration: none;
                    }
                    .nav-link::before {
                      content: '';
                      position: absolute;
                      bottom: 0;
                      left: 0;
                      width: 0;
                      height: 2px;
                      background-color: rgba(255, 253, 246, 0.8); /* primaryText/80 */
                      transition: width 300ms ease;
                    }
                    .nav-link:hover::before {
                      width: 100%;
                    }
                `}
                </style>

            </div>
                <div className="flex flex-row gap-1.5 md:gap-2 font-bold text-primaryText items-center">
                    <GitHubStarsButton className={"hidden sm:flex under-development bg-muted-foreground/10 border h-8 md:h-10 text-foreground text-xs md:text-sm"} username="Dev-log" repo="animate-ui"/>
                    <Button variant={"default"} className={"h-8 md:h-10 bg-bluePrimary text-white text-xs md:text-sm px-3 md:px-4"} onClick={onAuthenticate}>Sign in</Button>
                </div>
            </div>
        </div>
    )
}
