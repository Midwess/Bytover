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
                       <div className={"flex flex-row gap-2"}>
                           <Button className={"flex flex-row gap-3"}>
                               <img src={"/android.svg"} alt={"android"} className={"w-6 h-6"}/>
                                Android
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"w-6 h-6"}/>
                                iOS
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                <img src={"/windows.svg"} alt={"windows"} className={"w-6 h-6"}/>
                                Windows
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                <img src={"/apple.svg"} alt={"apple"} className={"w-6 h-6"}/>
                                Mac OS
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
                cursorSize={120}
                isViscous={false}
                viscous={40}
                iterationsViscous={32}
                iterationsPoisson={32}
                resolution={0.3}
                isBounce={false}
                autoDemo={true}
                autoSpeed={2.0}
                autoIntensity={1.0}
                takeoverDuration={0.25}
                autoResumeDelay={1000}
                autoRampDuration={0.9}
            />
        </div>
    </>
}
