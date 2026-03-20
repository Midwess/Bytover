import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import core from "@/core.ts"
import {motion, AnimatePresence} from "motion/react"
import {Button} from "@/components/ui/button.tsx";
import {invoke} from "@tauri-apps/api/core";
import {MousePointer2, Keyboard, Image as ImageIcon} from "lucide-react";
import {startDrag} from "@crabnebula/tauri-plugin-drag";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

function Window() {
    const [step, setStep] = useState(0);

    useEffect(() => {
        core.launch()
    }, [])

    const handleNext = () => {
        setStep((prev) => prev + 1)
    }

    const handleFinish = () => {
        invoke("hide_intro")
    }

    const handleDragStart = async (e: React.DragEvent) => {
        e.preventDefault();
        
        try {
            // Get the absolute path for the resource file from Tauri bundle
            const filePath = await invoke<string>("get_resource_path", { path: "icon.png" });
            
            await startDrag({
                item: [filePath],
                icon: filePath,
            });
        } catch (err) {
            console.error("Failed to start drag:", err);
        }
    }

    return (
        <main className="relative w-screen h-screen dark bg-background flex flex-col select-none overflow-hidden border border-white/5">
            <AnimatePresence mode="wait">
                {step === 0 ? (
                    <motion.div 
                        key="video-step-1"
                        initial={{ opacity: 0 }}
                        animate={{ opacity: 1 }}
                        exit={{ opacity: 0, x: -20 }}
                        className="flex flex-col h-full w-full"
                    >
                        {/* Top Part: Video Demo (70%) */}
                        <section 
                            data-tauri-drag-region
                            className="relative w-full h-[70%] bg-black flex items-center justify-center overflow-hidden cursor-default"
                        >
                            <video 
                                autoPlay 
                                loop 
                                muted 
                                playsInline
                                className="w-full h-full object-contain opacity-95 pointer-events-none"
                            >
                                <source src="/demo/demo-quick-share.mp4" type="video/mp4" />
                                Your browser does not support the video tag.
                            </video>
                            <div className="absolute inset-0 bg-gradient-to-t from-black/20 to-transparent pointer-events-none" />
                        </section>

                        {/* Bottom Part: Description and Action (30%) */}
                        <section className="relative flex-1 bg-[#1a1c1e] flex flex-col items-center justify-center py-8 px-16 border-t border-white/5 gap-6">
                            <motion.div 
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.2, duration: 0.5 }}
                                className="max-w-[440px] text-center"
                            >
                                <h2 className="text-white text-lg font-semibold mb-2 text-blue-400">Step 1: Quick Share</h2>
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
                    </motion.div>
                ) : step === 1 ? (
                    <motion.div 
                        key="interactive-step"
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0, x: -20 }}
                        className="flex flex-col h-full w-full"
                    >
                        {/* Top Part: Sample Image File (70%) */}
                        <section 
                            data-tauri-drag-region
                            className="relative w-full h-[70%] bg-[#121417] flex flex-col items-center justify-center overflow-hidden cursor-default gap-12"
                        >
                            <motion.div
                                animate={{ 
                                    y: [0, -10, 0],
                                    rotate: [0, -1, 1, 0]
                                }}
                                transition={{ 
                                    duration: 5, 
                                    repeat: Infinity,
                                    ease: "easeInOut"
                                }}
                                className="cursor-grab active:cursor-grabbing no-drag"
                            >
                                <div
                                    draggable
                                    onDragStart={handleDragStart}
                                    className="no-drag"
                                >
                                    <div className="flex flex-col items-center gap-4 group no-drag pointer-events-none">
                                        <div className="w-52 h-52 rounded-[40px] bg-white/5 border border-white/15 p-2 shadow-2xl relative overflow-hidden backdrop-blur-md transition-all group-hover:border-blue-500/30 group-hover:bg-white/10 no-drag pointer-events-none">
                                            <img 
                                                src="/icon.png" 
                                                alt="Bytover Icon" 
                                                className="w-full h-full object-contain p-8 rounded-[32px] pointer-events-none"
                                            />
                                            <div className="absolute top-5 right-5 bg-black/60 backdrop-blur-lg rounded-xl p-2 border border-white/10 shadow-lg pointer-events-none">
                                                <ImageIcon className="w-5 h-5 text-blue-400" />
                                            </div>
                                            <div className="absolute inset-0 bg-blue-500/5 opacity-0 group-hover:opacity-100 transition-opacity pointer-events-none" />
                                        </div>
                                        <div className="flex flex-col items-center gap-0.5 transition-transform group-hover:scale-105 pointer-events-none">
                                            <p className="text-white font-semibold text-[15px] tracking-tight">bytover_icon.png</p>
                                            <p className="text-white/40 text-[11px] font-medium tracking-wide uppercase">42 KB • PNG Image</p>
                                        </div>
                                    </div>
                                </div>
                            </motion.div>

                            <div className="flex gap-10 items-center text-white/20">
                                <div className="flex flex-col items-center gap-2">
                                    <div className="p-3.5 rounded-2xl border border-white/5 bg-white/5">
                                        <MousePointer2 className="w-5 h-5" />
                                    </div>
                                    <p className="text-[10px] uppercase tracking-[0.2em] font-bold">Shake mouse</p>
                                </div>
                                <div className="text-lg font-light opacity-30 italic">or</div>
                                <div className="flex flex-col items-center gap-2">
                                    <div className="p-3.5 rounded-2xl border border-white/5 bg-white/5">
                                        <Keyboard className="w-5 h-5" />
                                    </div>
                                    <p className="text-[10px] uppercase tracking-[0.2em] font-bold">Hold Shift</p>
                                </div>
                            </div>
                        </section>

                        {/* Bottom Part: Description and Action (30%) */}
                        <section className="relative flex-1 bg-[#1a1c1e] flex flex-col items-center justify-center py-6 px-16 border-t border-white/5 gap-4">
                            <motion.div 
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.2, duration: 0.5 }}
                                className="max-w-[440px] text-center"
                            >
                                <h2 className="text-white text-lg font-semibold mb-2 text-blue-400">Step 2: The Magic Gesture</h2>
                                <p className="text-[15px] leading-relaxed text-[#9ca3af] font-medium px-4">
                                    Try <span className="text-white">shaking your mouse</span> or <span className="text-white">holding Shift</span> while dragging this icon to open a shelf instantly.
                                </p>
                            </motion.div>

                            <motion.div
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.4, duration: 0.5 }}
                                className="w-full flex flex-col items-center gap-2"
                            >
                                <Button
                                    onClick={handleNext}
                                    className="min-w-[160px] h-10 bg-[#3b82f6] hover:bg-[#2563eb] text-white rounded-full text-sm font-semibold transition-all active:scale-[0.97] border-none shadow-none"
                                >
                                    I got it
                                </Button>
                                <button
                                    onClick={handleNext}
                                    className="text-[11px] text-[#9ca3af]/40 hover:text-[#9ca3af]/80 font-medium transition-colors uppercase tracking-widest"
                                >
                                    No, I don't care
                                </button>
                            </motion.div>
                        </section>
                    </motion.div>
                ) : (
                    <motion.div 
                        key="video-step-2"
                        initial={{ opacity: 0, x: 20 }}
                        animate={{ opacity: 1, x: 0 }}
                        exit={{ opacity: 0 }}
                        className="flex flex-col h-full w-full"
                    >
                        {/* Top Part: Video Demo Tray Menu (70%) */}
                        <section 
                            data-tauri-drag-region
                            className="relative w-full h-[70%] bg-black flex items-center justify-center overflow-hidden cursor-default"
                        >
                            <video 
                                autoPlay 
                                loop 
                                muted 
                                playsInline
                                className="w-full h-full object-contain opacity-95 pointer-events-none"
                            >
                                <source src="/demo/tray-menu-demo.mp4" type="video/mp4" />
                                Your browser does not support the video tag.
                            </video>
                            <div className="absolute inset-0 bg-gradient-to-t from-black/20 to-transparent pointer-events-none" />
                        </section>

                        {/* Bottom Part: Description and Action (30%) */}
                        <section className="relative flex-1 bg-[#1a1c1e] flex flex-col items-center justify-center py-8 px-16 border-t border-white/5 gap-6">
                            <motion.div 
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.2, duration: 0.5 }}
                                className="max-w-[440px] text-center"
                            >
                                <h2 className="text-white text-lg font-semibold mb-2 text-blue-400">Step 3: History & Recent</h2>
                                <p className="text-[15px] leading-relaxed text-[#9ca3af] font-medium px-4">
                                    Access your <span className="text-white">Recent Shelves</span> and histories directly from the tray menu at any time.
                                </p>
                            </motion.div>

                            <motion.div
                                initial={{ opacity: 0, y: 10 }}
                                animate={{ opacity: 1, y: 0 }}
                                transition={{ delay: 0.4, duration: 0.5 }}
                                className="w-full flex justify-center"
                            >
                                <Button
                                    onClick={handleFinish}
                                    className="min-w-[160px] h-11 bg-[#3b82f6] hover:bg-[#2563eb] text-white rounded-full text-sm font-semibold transition-all active:scale-[0.97] border-none shadow-none"
                                >
                                    Get Started
                                </Button>
                            </motion.div>
                        </section>
                    </motion.div>
                )}
            </AnimatePresence>
        </main>
    )
}

export default Window;
