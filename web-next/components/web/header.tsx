'use client'
import {GitHubStarsButton} from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantAuthenticate,
} from 'shared_types/types/shared_types'
import {Button} from "@/components/ui/button.tsx";

export default function Header() {


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

            </div>
                <div className="flex flex-row gap-1.5 md:gap-2 font-bold text-primaryText items-center">
                    <GitHubStarsButton className={"hidden sm:flex under-development bg-muted-foreground/10 border h-8 md:h-10 text-foreground text-xs md:text-sm"} username="Dev-log" repo="animate-ui"/>
                    <Button variant={"default"} className={"h-8 md:h-10 bg-bluePrimary text-white text-xs md:text-sm px-3 md:px-4"} onClick={() => core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantAuthenticate()))}
                    >Sign in</Button>
                </div>
            </div>
        </div>
    )
}
