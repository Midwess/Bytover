'use client';

import { getAssetUrl } from "@/utils/asset-url";
import Aurora from "@/components/Aurora";
import Link from "next/link";
import Image from "next/image";
import { useIsMobile } from "@/hooks/use-mobile";

interface IntroductionProps {
    disableBackground?: boolean;
    hidePrimaryButton?: boolean;
    header?: string;
}

export default function Introduction({
    disableBackground = false,
    hidePrimaryButton = false,
    header = "File transfer, made truly seamless",
}: IntroductionProps) {
    const containerClassName = disableBackground
        ? "w-full flex flex-col items-center justify-center"
        : "w-screen h-screen flex flex-col items-center justify-center";

    return (
        <div className={containerClassName}>
            {!disableBackground && (
                <div className="w-full h-screen absolute">
                    <GravityBackground />
                </div>
            )}
            <div className="relative z-20 flex flex-col w-full items-center justify-center px-6">
                <div className="flex flex-col items-center gap-6 md:gap-8 max-w-2xl text-center">
                    {/* Logo */}
                    <span className="inline-flex items-center gap-2 px-4 py-1.5 rounded-full text-sm font-semibold bg-white/10 text-white border border-white/20 backdrop-blur-sm">
                        <Image
                            src={getAssetUrl("/logo-color.svg")}
                            alt="Bytover"
                            width={20}
                            height={20}
                        />
                        Bytover
                    </span>

                    {/* Headline */}
                    <h1 className="text-4xl md:text-5xl lg:text-6xl font-bold tracking-tight text-white">
                        {header}
                    </h1>

                    {/* Description */}
                    <p className="text-lg md:text-xl text-foreground/60 max-w-md">
                        No upload required. Share directly between you and your friends.
                    </p>

                    {/* CTA */}
                    {!hidePrimaryButton && (
                        <Link
                            href="/transfer"
                            className="mt-4 inline-flex items-center gap-2 bg-bluePrimary hover:bg-bluePrimary/90 text-white font-semibold px-8 py-3 rounded-full transition-colors"
                        >
                            Start Sharing
                            <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5l7 7-7 7" />
                            </svg>
                        </Link>
                    )}
                </div>
            </div>
        </div>
    );
}

export function GravityBackground() {
    const isMobile = useIsMobile()
    return (
        <>
            <div className="w-screen md:pb-0 h-[95vh] md:h-[80vh] overflow-hidden absolute top-0 left-0 z-1 pointer-events-none">
                <Aurora
                    // Stronger blue palette
                    colorStops={[
                        "#1a4779", // deep sapphire, 10% lighter
                        "#1a6fc7", // vibrant blue, 10% lighter
                        "#3784ff", // strong vivid blue, 10% lighter
                        "#365ba1", // royal blue, 10% lighter
                        "#243f7f", // very deep blue, 10% lighter
                    ]}
                    blend={isMobile ? 0.25 : 0.50}
                    amplitude={isMobile ? 0.20 : 0.15}
                    speed={isMobile ? 1.5 : 1.0}
                />
            </div>
        </>
    );
}
