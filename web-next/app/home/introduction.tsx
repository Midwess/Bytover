'use client';

import { getAssetUrl } from "@/utils/asset-url";
import Aurora from "@/components/Aurora";
import { DownloadPlatforms } from "@/components/download-platforms";
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
    header = "Easy Peer to peer and Public file transfer",
}: IntroductionProps) {
    const containerClassName = disableBackground
        ? "w-full flex flex-col items-center justify-center"
        : "w-screen h-screen flex flex-col items-center justify-center";

    return <>
        <div className={containerClassName}>
            {!disableBackground && (
                <div className={"w-full h-screen absolute"}>
                    <GravityBackground />
                </div>
            )}
            <div className={'relative flex flex-col w-full items-center gap-4 md:gap-10 pb-8 md:pb-16 pt-20 md:pt-32 justify-center px-4'}>
                <div className={'flex flex-col items-center justify-center gap-12 md:gap-32 container z-2 w-full'}>
                    <div className={"flex flex-col items-center gap-8 md:gap-20"}>
                        <div className={"flex flex-col items-center gap-2 md:gap-4"}>
                            <div className={"flex bg-muted-foreground/10 py-1 px-2 rounded-lg flex-row gap-1 md:gap-2 text-2xl md:text-3xl lg:text-5xl font-black text-center items-center"}>
                                <Image src={getAssetUrl("/logo-color.svg")} alt={"logo"} width={40} height={40} className={"w-6 h-6 md:w-10 md:h-10"} />
                                <span className="whitespace-nowrap text-sm sm:text-md">Bytover</span>
                            </div>
                            <h1
                                className="text-4xl md:text-5xl lg:text-7xl font-black text-center h1 pointer-events-none px-2"
                            >{header}
                            </h1>
                        </div>
                        <div className={"flex flex-col gap-2 md:gap-3 items-center w-full px-4 md:px-0"}>
                            {!hidePrimaryButton && (
                                <Link href="/transfer" className={"rounded-lg flex flex-row gap-2 md:gap-3 bg-bluePrimary text-white font-bold text-sm md:text-base px-4 md:px-6 py-2 md:py-3"}>
                                    Try it now on web
                                </Link>
                            )}
                            <h2 className={"text-sm md:text-lg text-foreground/90"}>Available on many other platforms</h2>
                            <DownloadPlatforms />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </>
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
