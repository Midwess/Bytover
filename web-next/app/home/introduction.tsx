import {TypingText} from "@/components/animate-ui/text/typing.tsx";
import {WavyBackground} from "@/components/ui/wavy-background.tsx";
import {Button} from "@/components/ui/button.tsx";
import {Vortex} from "@/components/ui/vortex.tsx";

export default function Introduction() {
    return <>
        <div className={"w-screen h-screen relative flex flex-col items-center justify-center"}>
            <VortexBackground/>
            <WavyBackground containerClassName="max-w-4xl mx-auto h-[350px] text-center pt-32">
            </WavyBackground>
            <div className={'relative flex flex-col w-full items-center gap-10 pb-16 pt-32 justify-center'}>
                <div className={'flex flex-col items-center justify-center gap-32 container z-2 w-full'}>
                    <div className={"flex flex-col items-center gap-4"}>
                        <TypingText
                            delay={200}
                            duration={15}
                            className="text-7xl font-black text-center h1 pointer-events-none w-screen"
                            text="A seamless file transfer that you can trust"
                            cursor
                            cursorClassName="h-9"
                        />
                        <h3 className={"text-3xl text-muted-foreground"}>Redefine the way you sharing your files</h3>
                        <Button className={"flex flex-row gap-3"}>Experience right here on web</Button>
                        <h2 className={"text-lg text-primaryText/80"}>Or available on all platforms</h2>
                        <div className={"flex flex-row gap-2"}>
                            <Button className={"flex flex-row gap-3"}>
                                Android
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                iOS
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                Windows
                            </Button>
                            <Button className={"flex flex-row gap-3"}>
                                Mac OS
                            </Button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    </>
}

export function VortexBackground() {
    return (
        <div className="absolute z-20 w-[calc(100%-4rem)] mx-auto rounded-md  h-[30rem] overflow-hidden">
            <Vortex
                backgroundColor="transparent"
                className="flex items-center flex-col justify-center px-2 md:px-10 py-4 w-full h-full"
            >

            </Vortex>
        </div>
    );
}
