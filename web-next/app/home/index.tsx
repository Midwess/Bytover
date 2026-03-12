import Header from "@/components/web/header";
import Footer from "@/components/web/footer";

import { Suspense } from "react";
import Introduction from "@/app/home/introduction.tsx";
import { JoinWaitList } from "@/components/join-waitlist";
import { AdditionalFeatures } from "@/components/additional-features";
import { DesktopSection } from "@/components/desktop-section";
import { Pricing2 } from "@/components/pricing2";

export default function Home() {
    return (
        <div className="min-h-screen w-screen bg-background relative overflow-x-hidden selection:bg-bluePrimary selection:text-white font-inter">
            <Suspense fallback={null}>
                <Header className="px-3" />
            </Suspense>

            <main>
                <div id="intro">
                    <Introduction />
                </div>

                <div className="space-y-32 md:space-y-48">
                    <div id="desktop">
                        <DesktopSection />
                    </div>

                    <div id="pricing">
                        <Pricing2 />
                    </div>

                    <div id="more-features">
                        <AdditionalFeatures />
                    </div>

                    {/* Moved outside of container to allow full-width background */}
                    <div id="waitlist" className="w-full">
                        <JoinWaitList />
                    </div>
                </div>
            </main>

            <Footer className="bg-zinc-950 border-t border-white/5" />
        </div>
    );
}
