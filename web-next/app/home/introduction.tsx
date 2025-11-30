'use client';

import { TypingText } from "@/components/animate-ui/text/typing.tsx";
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
    header = "A seamless file transfer that you can trust",
}: IntroductionProps) {
    const isMobile = useIsMobile();
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
                            <div className={"rounded-xl md:rounded-2xl text-white px-2 md:px-3 py-0.5 font-bold gap-1.5 md:gap-2 backdrop-blur-2xl flex flex-row items-center justify-center border text-xs md:text-md"}>
                                <Image src={"/logo-color.svg"} alt={"logo"} width={40} height={40} className={"w-6 h-6 md:w-10 md:h-10"} />
                                <span className="whitespace-nowrap">Bytover</span>
                            </div>
                            {header && <TypingText
                                enableAnimation={!isMobile}
                                delay={200}
                                duration={15}
                                className="text-4xl md:text-5xl lg:text-7xl font-black text-center h1 pointer-events-none px-2"
                                text={header}
                                cursor
                                cursorClassName="h-6 md:h-9"
                            />}
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
    return (
        <>
            <div className="w-screen md:pb-0 h-[95vh] md:h-[80vh] overflow-hidden absolute top-0 left-0 z-1 pointer-events-none">
                <Aurora
                    // Stronger blue palette
                    colorStops={[
                        "#00336a", // deep sapphire
                        "#005fc1", // vibrant blue
                        "#2176ff", // strong vivid blue
                        "#2049a8", // royal blue
                        "#0a2873", // very deep blue
                    ]}
                    blend={0.50}
                    amplitude={0.15}
                    speed={1.0}
                />
            </div>
        </>
    );
}
