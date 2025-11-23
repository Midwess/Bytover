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
            <div className={'relative flex flex-col w-full items-center gap-10 pb-16 pt-32 justify-center'}>
                <div className={'flex flex-col items-center justify-center gap-32 container z-2 w-full'}>
                    <div className={"flex flex-col items-center gap-20"}>
                        <div className={"flex flex-col items-center gap-4"}>
                            <div className={"rounded-2xl text-white px-3 py-0.5 font-bold gap-2 backdrop-blur-2xl flex flex-row items-center justify-center border text-md"}>
                                <img src={"/logo-color.svg"} alt={"logo"} className={"w-10 h-10"}/>
                                Bytover
                            </div>
                        <TypingText
                            delay={200}
                            duration={15}
                            className="text-7xl font-black text-center h1 pointer-events-none max-w-[800px]"
                            text={"A seamless file transfer that you can trust"}
                            cursor
                            cursorClassName="h-9"
                        />
                        </div>
                        <div className={"flex flex-col gap-3 items-center"}>
                        <Button className={"flex flex-row gap-3"}>Try it now on web</Button>
                        <h2 className={"text-lg text-muted-foreground"}>Available on many other platforms</h2>
                       <div className={"flex flex-row items-center justify-center gap-0 bg-white rounded-xl border border-gray-200 px-2 py-1 shadow-sm"}>
                           <Button className={"flex flex-row items-center gap-2 bg-transparent hover:bg-white/10 px-4 py-2 rounded-lg transition-colors"}>
                               <img src={"/android.svg"} alt={"android"} className={"w-5 h-5"}/>
                                <span className="text-sm font-medium">Android</span>
                            </Button>
                            <div className="h-6 w-px bg-gray-200 mx-1" />
                            <Button className={"flex flex-row items-center gap-2 bg-transparent hover:bg-gray-100 px-4 py-2 rounded-lg transition-colors"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"w-5 h-5"}/>
                                <span className="text-sm font-medium">iOS</span>
                            </Button>
                            <div className="h-6 w-px bg-gray-200 mx-1" />
                            <Button className={"flex flex-row items-center gap-2 bg-transparent hover:bg-gray-100 px-4 py-2 rounded-lg transition-colors"}>
                                <img src={"/windows.svg"} alt={"windows"} className={"w-5 h-5"}/>
                                <span className="text-sm font-medium">Windows</span>
                            </Button>
                            <div className="h-6 w-px bg-gray-200 mx-1" />
                            <Button className={"flex flex-row items-center gap-2 bg-transparent hover:bg-gray-100 px-4 py-2 rounded-lg transition-colors"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"w-5 h-5"}/>
                                <span className="text-sm font-medium">Mac OS</span>
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
                colors={[ '#5227FF', '#FF9FFC', '#B19EEF' ]}
                mouseForce={30}
                cursorSize={90}
                isViscous={false}
                viscous={20}
                iterationsViscous={20}
                iterationsPoisson={26}
                resolution={0.4}
                isBounce={false}
                autoDemo={true}
                autoSpeed={0.5}
                autoIntensity={0.9}
                takeoverDuration={0.15}
                autoResumeDelay={500}
                autoRampDuration={0.4}
            />
        </div>
    </>
}
