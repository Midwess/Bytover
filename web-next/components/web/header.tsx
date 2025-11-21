'use client'
import {GitHubStarsButton} from '@/components/animate-ui/buttons/github-stars'
import Image from "next/image";
import {
    AppEventVariantAuthentication,
    AuthenticationEventVariantAuthenticate,
} from 'shared_types/types/shared_types'
import {Button} from "@/components/ui/button.tsx";

export default function Header() {
    const onAuthenticate= () => {
        core.update(new AppEventVariantAuthentication(new AuthenticationEventVariantAuthenticate()))
    }

    return (
        <div className="z-2 relative flex justify-between items-center w-full py-10 container">
            <div className="flex flex-row gap-2 items-center">
                <Image
                    width={35}
                    height={35}
                    src="logo.svg"
                    alt="Logo"
                />
            </div>

            <div className="absolute left-1/2 transform -translate-x-1/2">
                <div className="flex flex-row gap-5 rounded-full border border-primaryText/30 px-8 py-2">
                    {["About", "Pricing", "How it works"].map((item) => (
                        <a
                            key={item}
                            href="#"
                            className="nav-link text-primaryText/80"
                        >
                            <h2>{item}</h2>
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
                <GitHubStarsButton className={"bg-muted-foreground/10 border h-10 text-foreground"} username="Dev-log" repo="animate-ui"/>
                <Button variant={"default"} className={"h-10 bg-bluePrimary/70 border border-bluePrimary text-foreground"} onClick={onAuthenticate}>Join now</Button>
            </div>
        </div>
    )
}
