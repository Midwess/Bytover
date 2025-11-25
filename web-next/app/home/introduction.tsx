'use client';

import {TypingText} from "@/components/animate-ui/text/typing.tsx";
import {Button} from "@/components/ui/button.tsx";
import LiquidEther from "@/components/LiquidEther";
import { useState } from "react";
import { AnimatePresence, motion } from "motion/react";

export default function Introduction() {
    const [expandedPlatform, setExpandedPlatform] = useState<string | null>(null);

    const platforms = [
        { id: 'android', label: 'Android', icon: '/android.svg' },
        { id: 'ios', label: 'iOS', icon: '/apple.svg' },
        { id: 'windows', label: 'Windows', icon: '/windows.svg' },
        { id: 'macos', label: 'Mac OS', icon: '/apple.svg' },
    ];

    const handlePlatformClick = (platformId: string) => {
        setExpandedPlatform(expandedPlatform === platformId ? null : platformId);
    };

    const scrollToWaitlist = () => {
        const element = document.querySelector('#waitlist');
        if (element) {
            element.scrollIntoView({ behavior: 'smooth', block: 'start' });
        }
    };
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
                       <div className={"inline-flex flex-col items-stretch gap-0 bg-white rounded-lg md:rounded-xl border border-gray-200 px-1 md:px-2 py-0.5 md:py-1 shadow-sm overflow-hidden"}>
                           <div className={"flex flex-row items-center justify-center shrink-0"}>
                               {platforms.map((platform, index) => (
                                   <div key={platform.id} className="flex flex-row items-center shrink-0">
                                       <Button 
                                           onClick={() => handlePlatformClick(platform.id)}
                                           className={`flex flex-row items-center gap-1 md:gap-2 bg-transparent hover:bg-black/10 px-2 md:px-4 py-1.5 md:py-2 rounded-lg transition-colors shrink-0 ${expandedPlatform === platform.id ? 'bg-black/10' : ''}`}
                                       >
                                           <img 
                                               src={platform.icon} 
                                               alt={platform.id} 
                                               className={"hidden md:block w-4 h-4 md:w-5 md:h-5 shrink-0"}
                                           />
                                           <span className="text-xs md:text-sm font-medium whitespace-nowrap">{platform.label}</span>
                                       </Button>
                                       {index < platforms.length - 1 && (
                                           <div className="h-5 md:h-6 w-px bg-gray-200 mx-0.5 md:mx-1 shrink-0" />
                                       )}
                                   </div>
                               ))}
                           </div>
                           <AnimatePresence>
                               {expandedPlatform && (
                                   <motion.div
                                       initial={{ opacity: 0, height: 0 }}
                                       animate={{ opacity: 1, height: 'auto' }}
                                       exit={{ opacity: 0, height: 0 }}
                                       transition={{ 
                                           duration: 0.5, 
                                           ease: [0.4, 0, 0.2, 1],
                                           height: { duration: 0.5, ease: [0.4, 0, 0.2, 1] },
                                           opacity: { duration: 0.4, ease: 'easeInOut' }
                                       }}
                                       className="overflow-hidden w-full"
                                       style={{ width: '100%', maxWidth: '100%' }}
                                   >
                                       <div className="pt-2 md:pt-3 px-2 md:px-4 pb-1 md:pb-2 text-center">
                                           <p className="text-xs md:text-sm text-gray-600 mb-1">
                                               We&apos;re currently developing native versions
                                           </p>
                                           <p className="text-xs md:text-sm text-gray-600 mb-2 md:mb-3">
                                               and will release soon this year.{' '}
                                               <button
                                                   onClick={scrollToWaitlist}
                                                   className="text-bluePrimary hover:text-blue-600 underline transition-colors"
                                               >
                                                   Join the waitlist
                                               </button>
                                           </p>
                                       </div>
                                   </motion.div>
                               )}
                           </AnimatePresence>
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
        <div className={"w-full h-full absolute top-0 left-0 z-1 pointer-events-none"}>
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
                autoIntensity={1.8}
                takeoverDuration={0.15}
                autoResumeDelay={100}
                autoRampDuration={0.4}
            />
        </div>
    </>
}
