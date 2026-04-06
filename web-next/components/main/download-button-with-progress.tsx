'use client'

import {Check, ArrowDown} from 'lucide-react'
import {useState, useEffect, useRef} from 'react'
import {cn} from '@/lib/utils.ts'
import { motion } from 'framer-motion'

interface DownloadButtonWithProgressProps {
    progress?: number
    isReady?: boolean
    isCompleted?: boolean
    isInProgress?: boolean
    isCloud?: boolean
    onDownloadClick?: () => void
    onCancelClick?: () => void
    className?: string
    buttonText?: string
    buttonVariant?: 'default' | 'outline' | 'ghost'
    buttonSize?: 'default' | 'sm' | 'lg' | 'icon'
    containerClass?: string
    speed?: string
}

export default function DownloadButtonWithProgress({
    progress = 0,
    isReady = true,
    isCompleted = false,
    isInProgress = false,
    isCloud = false,
    onDownloadClick = () => {},
    onCancelClick = () => {},
    className = '',
    buttonText,
    containerClass = '',
    speed
}: DownloadButtonWithProgressProps) {
    const [showCompleted, setShowCompleted] = useState(false)
    const wasInProgressRef = useRef(false)

    useEffect(() => {
        if (isInProgress) {
            wasInProgressRef.current = true
        }

        if (isCompleted && wasInProgressRef.current) {
            requestAnimationFrame(() => setShowCompleted(true))
            const timer = setTimeout(() => {
                setShowCompleted(false)
                wasInProgressRef.current = false
            }, 1000)
            return () => clearTimeout(timer)
        } else if (!isInProgress && !isCompleted) {
            wasInProgressRef.current = false
            requestAnimationFrame(() => setShowCompleted(false))
        }
    }, [isCompleted, isInProgress])

    const getButtonState = () => {
        if (showCompleted) return 'completed'
        if (isInProgress) return 'downloading'
        if (isCloud && progress === 0 && !isCompleted) return 'waiting'
        return 'idle'
    }

    const state = getButtonState()

    const isPill = !!buttonText;
    const commonClasses = cn(
        isPill ? "h-12 px-8 w-auto min-w-[180px]" : "w-12 h-12",
        "rounded-full flex items-center justify-center transition-all duration-500",
        containerClass
    )

    return (
        <div className={cn("relative flex flex-col items-center justify-center", className)}>
            {state === 'idle' && (
                <button
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className={cn(
                        commonClasses,
                        "bg-transparent border border-white/5 text-white/40 gap-3",
                        "hover:bg-white hover:text-black hover:border-white disabled:opacity-20 disabled:cursor-not-allowed"
                    )}
                >
                    <ArrowDown className={isPill ? "w-4 h-4" : "w-5 h-5"} strokeWidth={2.5} />
                    {buttonText && <span className="text-[11px] font-bold uppercase tracking-[0.2em] whitespace-nowrap">{buttonText}</span>}
                </button>
            )}

            {state === 'downloading' && (
                <button
                    onClick={onCancelClick}
                    className={cn(
                        isPill ? "h-12 px-6 min-w-[200px]" : "w-12 h-12",
                        "rounded-full bg-white/5 border border-white/10 relative overflow-hidden flex flex-col items-center justify-center transition-all duration-500"
                    )}
                    title="Cancel download"
                >
                    {!isPill ? (
                        <>
                            <svg className="absolute inset-0 w-full h-full -rotate-90" viewBox="0 0 40 40">
                                <circle cx="20" cy="20" r="18" stroke="currentColor" strokeWidth="2.5" fill="transparent" className="text-white/5" />
                                <circle cx="20" cy="20" r="18" stroke="currentColor" strokeWidth="2.5" fill="transparent" strokeDasharray={113.1} strokeDashoffset={113.1 - (113.1 * progress)} strokeLinecap="round" className="text-white transition-all duration-300" />
                            </svg>
                            <span className="relative z-10 text-xs font-bold text-white tabular-nums">{Math.round(progress * 100)}%</span>
                        </>
                    ) : (
                        <div className="w-full space-y-1.5">
                            <div className="flex items-center justify-between px-1">
                                <span className="text-[9px] font-bold text-white uppercase tracking-widest">{speed || 'Downloading'}</span>
                                <span className="text-[9px] font-bold text-white tabular-nums">{Math.round(progress * 100)}%</span>
                            </div>
                            <div className="h-1 w-full bg-white/10 rounded-full overflow-hidden">
                                <motion.div 
                                    initial={false}
                                    animate={{ width: `${progress * 100}%` }}
                                    className="h-full bg-white"
                                />
                            </div>
                        </div>
                    )}
                </button>
            )}

            {state === 'waiting' && (
                <div className={cn(commonClasses, "bg-white/5 border border-white/10 animate-pulse")}>
                    <span className="text-xs font-bold text-zinc-500 uppercase tracking-widest">...</span>
                </div>
            )}

            {state === 'completed' && (
                <div className={cn(commonClasses, "bg-white text-black shadow-[0_0_20px_rgba(255,255,255,0.4)]")}>
                    <Check className="w-5 h-5" strokeWidth={3} />
                </div>
            )}

            {state === 'downloading' && (
                <span className="text-[9px] text-white/40 font-medium mt-1.5">Press to cancel</span>
            )}
        </div>
    )
}
