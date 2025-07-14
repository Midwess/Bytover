import {StarsBackground} from "@/components/animate-ui/backgrounds/stars";
import {TypingText} from "@/components/animate-ui/text/typing";
import Header from "@/components/web/header";
import {LiquidButton} from '@/components/animate-ui/buttons/liquid'
import Android from '@/public/android.svg'
import apple from '@/public/apple.svg'
import windows from '@/public/windows.svg'
import Image from 'next/image'
import TransferBoard from "@/app/transfer";

export default function Home() {
    return <div className="flex flex-col w-full h-full items-center">
        <div className={'relative flex flex-col w-full items-center gap-10 pb-16'}>
            <Header/>
            <div
                className="absolute top-0 z-[-1] h-full w-screen bg-blackBase bg-[radial-gradient(ellipse_80%_80%_at_50%_-20%,rgba(124,255,121,0.2),rgba(255,255,255,0))]">
            </div>
            <div className={'flex flex-col items-center gap-4 container'}>
                <h2 className="text-lg tracking-widest  text-greenSecondary text-center">
                    Powering your productivity 👋
                </h2>
                <TypingText
                    delay={200}
                    duration={15}
                    className="text-5xl font-bold text-center h1"
                    text="Seamless file transfer that you can trust"
                    cursor
                    cursorClassName="h-9"
                />
            </div>
            <div className={"flex flex-col items-center gap-4"}>
                <h2 className={"font-bold text-lg text-primaryText/80"}>Available on all platforms</h2>
                <div className={"flex flex-row gap-2"}>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={Android} alt="Android" width={20} height={20}/>
                        Android
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={apple} alt="iOS" width={20} height={20}/>
                        iOS
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={windows} alt="Windows" width={20} height={20}/>
                        Windows
                    </LiquidButton>
                    <LiquidButton className={"flex flex-row gap-3"}>
                        <Image src={apple} alt="Mac" width={20} height={20}/>
                        Mac OS
                    </LiquidButton>
                </div>
            </div>
        </div>
        <div className={"container flex flex-col mt-10"}>
            <TransferBoard/>
        </div>
        <div className={"h-36 w-full"}></div>
    </div>
}