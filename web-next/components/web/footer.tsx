'use client'
import Image from "next/image";
import Link from "next/link";

export default function Footer() {
    const scrollToSection = (href: string) => {
        const element = document.querySelector(href);
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };

    return (
        <footer className="w-full bg-black border-t border-zinc-800">
            <div className="container mx-auto px-4 md:px-6 py-12 md:py-16">
                <div className="flex flex-col md:flex-row justify-between gap-8 md:gap-12">
                    {/* Brand Section */}
                    <div className="flex flex-col gap-3">
                        <div className="flex items-center gap-3">
                            <Image
                                width={40}
                                height={40}
                                src="/logo.svg"
                                alt="Logo"
                                className="rounded-lg aspect-square w-10 h-10"
                            />
                            <h2 className="text-xl font-bold text-primaryText">Bytover</h2>
                        </div>
                        <p className="text-muted-foreground text-sm md:text-base">
                            A seamless file transfer that you can trust
                        </p>
                    </div>

                    {/* Links Section */}
                    <div className="flex flex-col md:flex-row gap-8 md:gap-12">
                        {/* Sections */}
                        <div className="flex flex-col gap-3">
                            <h3 className="font-semibold text-primaryText text-sm md:text-base">Sections</h3>
                            <ul className="flex flex-col gap-2">
                                <li>
                                    <a
                                        href="#intro"
                                        onClick={(e) => {
                                            e.preventDefault();
                                            scrollToSection('#intro');
                                        }}
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Home
                                    </a>
                                </li>
                                <li>
                                    <a
                                        href="#features"
                                        onClick={(e) => {
                                            e.preventDefault();
                                            scrollToSection('#features');
                                        }}
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Features
                                    </a>
                                </li>
                                <li>
                                    <a
                                        href="#pricing"
                                        onClick={(e) => {
                                            e.preventDefault();
                                            scrollToSection('#pricing');
                                        }}
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Pricing
                                    </a>
                                </li>
                                <li>
                                    <a
                                        href="#waitlist"
                                        onClick={(e) => {
                                            e.preventDefault();
                                            scrollToSection('#waitlist');
                                        }}
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Join Waitlist
                                    </a>
                                </li>
                            </ul>
                        </div>

                        {/* Support */}
                        <div className="flex flex-col gap-3">
                            <h3 className="font-semibold text-primaryText text-sm md:text-base">Support</h3>
                            <ul className="flex flex-col gap-2">
                                <li>
                                    <Link
                                        href="#"
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        FAQ
                                    </Link>
                                </li>
                                <li>
                                    <Link
                                        href="#"
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Contact
                                    </Link>
                                </li>
                                <li>
                                    <Link
                                        href="#"
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Privacy Policy
                                    </Link>
                                </li>
                                <li>
                                    <Link
                                        href="#"
                                        className="text-muted-foreground hover:text-primaryText transition-colors text-sm md:text-base"
                                    >
                                        Terms
                                    </Link>
                                </li>
                            </ul>
                        </div>
                    </div>
                </div>
            </div>
        </footer>
    );
}

