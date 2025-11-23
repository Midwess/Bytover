'use client';

import {TypingText} from "@/components/animate-ui/text/typing.tsx";
import {Button} from "@/components/ui/button.tsx";
import LiquidEther from "@/components/LiquidEther";

export default function Introduction() {
    return <>
        <div className={"w-screen h-screen flex flex-col items-center justify-center"}>
            <div className={"w-full h-screen absolute"}>
                <GravityBackground/>
            </div>
            <div className={'relative flex flex-col w-full items-center gap-4 md:gap-10 pb-8 md:pb-16 pt-20 md:pt-32 justify-center px-4'}>
                <div className={'flex flex-col items-center justify-center gap-12 md:gap-32 container z-2 w-full'}>
                    <div className={"flex flex-col items-center gap-8 md:gap-20"}>
                        <div className={"flex flex-col items-center gap-2 md:gap-4"}>
                            <div className={"rounded-xl md:rounded-2xl text-white px-2 md:px-3 py-0.5 font-bold gap-1.5 md:gap-2 backdrop-blur-2xl flex flex-row items-center justify-center border text-xs md:text-md"}>
                                <img src={"/logo-color.svg"} alt={"logo"} className={"w-6 h-6 md:w-10 md:h-10"}/>
                                <span className="whitespace-nowrap">Bytover</span>
                            </div>
                        <TypingText
                            delay={200}
                            duration={15}
                            className="text-4xl md:text-5xl lg:text-7xl font-black text-center h1 pointer-events-none px-2"
                            text={"A seamless file transfer that you can trust"}
                            cursor
                            cursorClassName="h-6 md:h-9"
                        />
                        </div>
                        <div className={"flex flex-col gap-2 md:gap-3 items-center w-full px-4 md:px-0"}>
                        <Button className={"flex flex-row gap-2 md:gap-3 bg-bluePrimary text-white font-bold text-sm md:text-base px-4 md:px-6 py-2 md:py-3"}>Try it now on web</Button>
                        <h2 className={"text-sm md:text-lg text-foreground/90"}>Available on many other platforms</h2>
                       <div className={"flex flex-row items-center justify-center gap-0 bg-white rounded-lg md:rounded-xl border border-gray-200 px-1 md:px-2 py-0.5 md:py-1 shadow-sm w-fit max-w-full"}>
                           <Button className={"flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/30 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors"}>
                               <img src={"/android.svg"} alt={"android"} className={"hidden md:block w-4 h-4 md:w-5 md:h-5"}/>
                                <span className="text-xs md:text-sm font-medium">Android</span>
                            </Button>
                            <div className="h-5 md:h-6 w-px bg-gray-200 mx-0.5 md:mx-1" />
                            <Button className={"flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/30 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"hidden md:block w-4 h-4 md:w-5 md:h-5"}/>
                                <span className="text-xs md:text-sm font-medium">iOS</span>
                            </Button>
                            <div className="h-5 md:h-6 w-px bg-gray-200 mx-0.5 md:mx-1" />
                            <Button className={"flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/30 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors"}>
                                <img src={"/windows.svg"} alt={"windows"} className={"hidden md:block w-4 h-4 md:w-5 md:h-5"}/>
                                <span className="text-xs md:text-sm font-medium">Windows</span>
                            </Button>
                            <div className="h-5 md:h-6 w-px bg-gray-200 mx-0.5 md:mx-1" />
                            <Button className={"flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/30 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"hidden md:block w-4 h-4 md:w-5 md:h-5"}/>
                                <span className="text-xs md:text-sm font-medium">Mac OS</span>
                            </Button>
                        </div>
                    </div>
                    </div>
                </div>
            </div>
        </div>
    </>
}

export function GravityBackground() {
    return <>
        <div style={{ width: '100%', height: '100%', position: 'relative' }}>
            <LiquidEther
                colors={[ '#3B82F6', '#60A5FA', '#93C5FD' ]}
                mouseForce={30}
                cursorSize={86}
                isViscous={false}
                viscous={20}
                iterationsViscous={20}
                iterationsPoisson={26}
                resolution={0.4}
                isBounce={false}
                autoDemo={true}
                autoSpeed={0.3}
                autoIntensity={0.9}
                takeoverDuration={0.15}
                autoResumeDelay={500}
                autoRampDuration={0.4}
            />
        </div>
    </>
}
