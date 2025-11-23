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
            setIsAtTop(currentScrollY < 50);
            
            if (currentScrollY < 10) {
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
        <div className={`fixed top-0 left-0 right-0 z-50 flex justify-between items-center w-full py-6 px-4 transition-all duration-300 ${isVisible ? 'translate-y-0' : '-translate-y-full'} ${isAtTop ? 'bg-transparent border-b border-transparent shadow-none' : 'backdrop-blur-xl bg-blackBase/90 border-b border-white/10 shadow-[0_8px_32px_0_rgba(0,0,0,0.37)]'}`}>
            <div className="container mx-auto flex justify-between items-center">
            <div className="flex flex-row gap-2 items-center">
                {/*<Image*/}
                {/*    className={"bg-bluePrimary/40 border-2 border-bluePrimary/50 rounded-lg aspect-square p-1.5"}*/}
                {/*    width={55}*/}
                {/*    height={55}*/}
                {/*    src="logo.svg"*/}
                {/*    alt="Logo"*/}
                {/*/>*/}
            </div>

            <div className="absolute left-1/2 transform -translate-x-1/2">
                <div className="flex flex-row gap-5 rounded-xl border border-primaryText/30 px-8 py-2">
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
                            className="nav-link text-primaryText/80"
                        >
                            <h2>{item.label}</h2>
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
                <div className="flex flex-row gap-2 font-bold text-primaryText">
                    <GitHubStarsButton className={"under-development bg-muted-foreground/10 border h-10 text-foreground"} username="Dev-log" repo="animate-ui"/>
                    <Button variant={"default"} className={"h-10 bg-bluePrimary/70 border border-bluePrimary text-foreground"} onClick={onAuthenticate}>Join now</Button>
                </div>
            </div>
        </div>
    )
}
