import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import core from "@/core.ts"
import {motion, noop} from "motion/react"
import {Button} from "@/components/ui/button.tsx";
import {Input} from "@/components/ui/input.tsx";
import Iridescence from "@/components/iridescene.tsx";
import {invoke} from "@tauri-apps/api/core";
import {listen} from "@tauri-apps/api/event";
import {openUrl} from "@tauri-apps/plugin-opener";
import {ArrowUpRight, Check, Copy} from "lucide-react";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <Window/>
    </React.StrictMode>,
);

type AuthPhase = 'google-signin' | 'token-input' | 'loading';

function Window() {
    const [authPhase, setAuthPhase] = useState<AuthPhase>('google-signin')
    const [tokenInput, setTokenInput] = useState('')
    const [authUrl, setAuthUrl] = useState<string | null>(null)
    const [copied, setCopied] = useState(false)

    useEffect(() => {
        core.launch()
    }, [])

    useEffect(() => {
        const unlistenPromise = listen<string>('auth-url', (event) => {
            setAuthUrl(event.payload)
        })
        return () => {
            unlistenPromise.then((unlisten) => unlisten())
        }
    }, [])

    const handleLogin = () => {
        if (authPhase !== 'google-signin') return
        setAuthPhase('token-input')
        invoke("authenticate").then(noop)
    }

    const handleSubmitToken = () => {
        if (!tokenInput.trim() || authPhase !== 'token-input') return
        setAuthPhase('loading')
        invoke("submit_token", { token: tokenInput.trim() }).then(noop)
        setTimeout(() => {
            setAuthPhase('token-input')
        }, 4000)
    }

    const handleBack = () => {
        if (authPhase !== 'token-input') return
        setTokenInput('')
        setAuthUrl(null)
        setCopied(false)
        setAuthPhase('google-signin')
    }

    const handleCopyUrl = async () => {
        if (!authUrl) return
        try {
            await navigator.clipboard.writeText(authUrl)
            setCopied(true)
            setTimeout(() => setCopied(false), 2000)
        } catch {
            setCopied(false)
        }
    }

    const handleOpenUrl = async () => {
        if (!authUrl) return
        await openUrl(authUrl).catch(() => {})
    }

    return (
        <main className="relative w-screen h-screen dark bg-blackBase flex flex-col select-none overflow-hidden border border-white/5">
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
                    className="w-full flex flex-col items-center gap-5"
                >
                    {authPhase === 'google-signin' && (
                        <>
                            <Button
                                onClick={handleLogin}
                                className="min-w-[240px] h-12 bg-white hover:bg-white/90 text-blackBase rounded-full text-[15px] font-semibold transition-all active:scale-[0.98] border-none shadow-lg flex items-center justify-center gap-3"
                            >
                                <svg width="20" height="20" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
                                    <path d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z" fill="#4285F4"/>
                                    <path d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z" fill="#34A853"/>
                                    <path d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l3.66-2.84z" fill="#FBBC05"/>
                                    <path d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 12-4.53z" fill="#EA4335"/>
                                </svg>
                                Sign in with Google
                            </Button>
                            <p className="text-[13px] text-[#9ca3af] text-center">
                                By signing in, you automatically accept our <span className="text-[#3b82f6] hover:underline cursor-pointer" onClick={() => openUrl('https://bytover.com/policy')}>policy</span>.
                            </p>
                        </>
                    )}

                    {authPhase === 'token-input' && (
                        <div className="w-full max-w-[320px] flex flex-col items-center gap-5">
                            <div className="text-center">
                                <h2 className="text-white text-[17px] font-semibold tracking-tight">
                                    Continue in your browser
                                </h2>
                                <p className="text-[13px] text-zinc-400 mt-1">
                                    Open the sign-in link, or paste the access token below.
                                </p>
                            </div>

                            {authUrl && (
                                <div className="w-full flex flex-col gap-2">
                                    <div className="w-full px-3.5 py-2.5 rounded-xl bg-white/[0.06] text-zinc-200 text-[12px] font-mono truncate">
                                        {authUrl}
                                    </div>
                                    <div className="flex gap-2">
                                        <button
                                            onClick={handleCopyUrl}
                                            className="flex-1 h-10 bg-white/[0.06] hover:bg-white/[0.1] text-white rounded-xl text-[13px] font-medium flex items-center justify-center gap-1.5 transition-colors active:scale-[0.97]"
                                        >
                                            {copied ? <Check className="w-[14px] h-[14px]" /> : <Copy className="w-[14px] h-[14px]" />}
                                            {copied ? 'Copied' : 'Copy'}
                                        </button>
                                        <button
                                            onClick={handleOpenUrl}
                                            className="flex-1 h-10 bg-white/[0.06] hover:bg-white/[0.1] text-white rounded-xl text-[13px] font-medium flex items-center justify-center gap-1.5 transition-colors active:scale-[0.97]"
                                        >
                                            <ArrowUpRight className="w-[14px] h-[14px]" />
                                            Open
                                        </button>
                                    </div>
                                </div>
                            )}

                            <div className="w-full flex items-center gap-3">
                                <div className="flex-1 h-px bg-white/10" />
                                <span className="text-[10px] text-zinc-500 uppercase tracking-[0.15em] font-medium">or</span>
                                <div className="flex-1 h-px bg-white/10" />
                            </div>

                            <Input
                                type="text"
                                placeholder="Paste access token"
                                value={tokenInput}
                                onChange={(e) => setTokenInput(e.target.value)}
                                className="w-full h-11 bg-white/[0.06] border-none text-white placeholder:text-zinc-500 rounded-xl text-[13px] px-3.5 focus-visible:ring-1 focus-visible:ring-white/20"
                            />
                            <Button
                                onClick={handleSubmitToken}
                                disabled={!tokenInput.trim()}
                                className="w-full h-11 bg-white hover:bg-white/90 text-blackBase rounded-full text-[14px] font-semibold transition-all active:scale-[0.98] disabled:opacity-40 disabled:cursor-not-allowed border-none"
                            >
                                Continue
                            </Button>
                            <button
                                onClick={handleBack}
                                className="text-[12px] text-zinc-500 hover:text-zinc-300 transition-colors"
                            >
                                Back
                            </button>
                        </div>
                    )}

                    {authPhase === 'loading' && (
                        <div className="flex flex-col items-center gap-3">
                            <div className="h-8 w-8 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                            <p className="text-[14px] text-[#9ca3af]">Authenticating...</p>
                        </div>
                    )}
                </motion.div>
            </section>
        </main>
    )
}

export default Window;
