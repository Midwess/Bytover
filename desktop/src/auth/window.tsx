import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import core from "@/core.ts"
import {motion, noop} from "motion/react"
import {Button} from "@/components/ui/button.tsx";
import Iridescence from "@/components/iridescene.tsx";
import {invoke} from "@tauri-apps/api/core";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

function Window() {
    const [isLoading, setIsLoading] = useState(false)

    useEffect(() => {
        core.launch()
    }, [])

    const handleLogin = () => {
        if (isLoading) return
        setIsLoading(true)
        invoke("authenticate").then(noop)
        setTimeout(() => setIsLoading(false), 10000)
    }

    return (
        <main className="relative w-screen h-screen dark bg-blackBase flex flex-col select-none overflow-hidden rounded-b-xl border border-white/5">
            {/* Top Part: Vibrant Visual Background and Title */}
            <section 
                data-tauri-drag-region
                className="relative w-full h-[58%] flex flex-col items-center justify-center overflow-hidden cursor-default"
            >
                <div className="absolute inset-0 z-0 pointer-events-none">
                    <Iridescence
                        color={[0.3, 0.5, 0.9]} 
                        mouseReact={true}
                        amplitude={0.05}
                        speed={0.6}
                    />
                </div>
                
                <div className="relative z-10 flex flex-col items-center gap-8 pointer-events-none">
                    <motion.div 
                        initial={{ opacity: 0, scale: 0.9 }}
                        animate={{ opacity: 1, scale: 1 }}
                        transition={{ duration: 0.8, ease: "easeOut" }}
                        className="w-32 h-32 bg-white/10 backdrop-blur-xl rounded-[24%] flex items-center justify-center border border-white/20 shadow-2xl"
                    >
                        <img src="/logo.svg" alt="Bytover Logo" className="w-20 h-20 object-contain brightness-110 drop-shadow-md" />
                    </motion.div>
                    
                    <motion.h1 
                        initial={{ opacity: 0, y: 10 }}
                        animate={{ opacity: 1, y: 0 }}
                        transition={{ delay: 0.2, duration: 0.6 }}
                        className="text-[36px] font-bold tracking-tight text-white leading-tight drop-shadow-lg text-center px-6"
                    >
                        Shelves with superpowers.
                    </motion.h1>
                </div>
            </section>

            {/* Bottom Part: Description and Action */}
            <section className="relative flex-1 bg-[#1a1c1e] flex flex-col items-center justify-center py-10 px-16 border-t border-white/5 gap-8">
                <motion.div 
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ delay: 0.4, duration: 0.6 }}
                    className="max-w-[440px] text-center flex flex-col gap-3"
                >
                    <p className="text-[18px] leading-relaxed text-[#9ca3af] font-medium">
                        Instant P2P sharing. No clouds, no zips, just magic.
                    </p>
                </motion.div>

                <motion.div
                    initial={{ opacity: 0, y: 10 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: 0.6, duration: 0.6 }}
                    className="w-full flex justify-center"
                >
                    <Button
                        onClick={handleLogin}
                        disabled={isLoading}
                        className="min-w-[150px] h-11 bg-[#3b82f6] hover:bg-[#2563eb] text-white rounded-full text-[15px] font-semibold transition-all active:scale-[0.97] border-none shadow-none"
                    >
                        {isLoading ? (
                            <div className="h-5 w-5 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                        ) : (
                            "Get Started"
                        )}
                    </Button>
                </motion.div>
            </section>
        </main>
    )
}

export default Window;
