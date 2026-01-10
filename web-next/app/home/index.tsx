import Header from "@/components/web/header";
import Footer from "@/components/web/footer";

import { Suspense } from "react";
import Introduction from "@/app/home/introduction.tsx";
import { JoinWaitList } from "@/components/join-waitlist";
import { AdditionalFeatures } from "@/components/additional-features";
import { DesktopSection } from "@/components/desktop-section";
import { Pricing2 } from "@/components/pricing2";


export default function Home() {
    return <div className="flex flex-col w-full h-full items-center bg-black">
        <Suspense fallback={null}>
            <Header className="px-6 sm:px-4 container" />
        </Suspense>

        <div id="intro" className={"w-screen h-screen bg-black"}>
            <Introduction />
        </div>

        <div id="desktop" className={"w-screen bg-zinc-900 pt-8"}>
            <div className="w-screen">
                <DesktopSection />
            </div>
        </div>

        <div id="pricing" className={"w-full"}>
            <Pricing2 />
        </div>

        <div id="more-features" className={"w-full bg-blue-800/10"}>
            <AdditionalFeatures />
        </div>

        <div id="waitlist" className={"w-full bg-zinc-900 h-[60vh] py-5 min-h-fit items-center flex"}>
            <JoinWaitList />
        </div>

        <Footer />
    </div>
}
