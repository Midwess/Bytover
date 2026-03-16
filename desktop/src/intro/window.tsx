import ReactDOM from "react-dom/client"
import React, {useEffect} from "react"
import core from "@/core.ts"
import {motion} from "motion/react"
import {Button} from "@/components/ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

function Window() {
    useEffect(() => {
        core.launch()
    }, [])

    const handleNext = () => {
        invoke("hide_intro")
    }

    return (
        <main className="relative w-screen h-screen dark bg-blackBase flex flex-col select-none overflow-hidden rounded-xl border border-white/5">
            {/* Top Part: Video Demo (70%) */}
            <section className="relative w-full h-[70%] bg-black flex items-center justify-center overflow-hidden">
                <video 
                    autoPlay 
                    loop 
                    muted 
                    playsInline
                    className="w-full h-full object-cover opacity-90"
                >
                    <source src="/demo/demo-quick-share.mp4" type="video/mp4" />
                    Your browser does not support the video tag.
                </video>
                {/* Subtle overlay to blend with the app aesthetic */}
                <div className="absolute inset-0 bg-gradient-to-t from-black/40 to-transparent pointer-events-none" />
            </section>

            {/* Bottom Part: Description and Action (30%) */}
            <section className="relative flex-1 bg-[#1a1c1e] flex flex-col items-center justify-center py-8 px-16 border-t border-white/5 gap-6">
                <motion.div 
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.2, duration: 0.5 }}
                    className="max-w-[440px] text-center"
                >
                    <h2 className="text-white text-lg font-semibold mb-2">Quick Share</h2>
                    <p className="text-[15px] leading-relaxed text-[#9ca3af] font-medium">
                        Instantly share files by dragging them onto the shelf. No cloud uploads, just pure P2P speed.
                    </p>
                </motion.div>

                <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.4, duration: 0.5 }}
                    className="w-full flex justify-center"
                >
                    <Button
                        onClick={handleNext}
                        className="min-w-[120px] h-10 bg-[#3b82f6] hover:bg-[#2563eb] text-white rounded-full text-sm font-semibold transition-all active:scale-[0.97] border-none shadow-none"
                    >
                        Next
                    </Button>
                </motion.div>
            </section>
        </main>
    )
}

export default Window;
