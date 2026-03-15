'use client'

import {Check, ArrowDown} from 'lucide-react'
import {Button} from '@/components/ui/button.tsx'
import {useState, useEffect, useRef} from 'react'
import {cn} from '@/lib/utils.ts'

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
    buttonSize = 'sm',
    containerClass = ''
}: DownloadButtonWithProgressProps) {
    const [showCompleted, setShowCompleted] = useState(false)
    const wasInProgressRef = useRef(false)

    useEffect(() => {
        if (isInProgress) {
            wasInProgressRef.current = true
        }

        if (isCompleted && wasInProgressRef.current) {
            setShowCompleted(true)
            const timer = setTimeout(() => {
                setShowCompleted(false)
                wasInProgressRef.current = false
            }, 1000)
            return () => clearTimeout(timer)
        } else if (!isInProgress && !isCompleted) {
            wasInProgressRef.current = false
            setShowCompleted(false)
        }
    }, [isCompleted, isInProgress])

    const getButtonState = () => {
        if (showCompleted) return 'completed'
        if (isInProgress) return 'downloading'
        if (isCloud && progress === 0 && !isCompleted) return 'waiting'
        return 'idle'
    }

    const state = getButtonState()

    const containerClassFinal = cn("h-8 w-36 rounded-lg transition-colors", containerClass)

    return (
        <div className={`relative ${className}`}>
            {state === 'idle' && !buttonText && (
                <Button
                    size="icon"
                    variant="ghost"
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className={`${containerClassFinal} bg-blue-600 hover:bg-blue-700`}
                >
                    <ArrowDown className="h-[50%] w-[50%] text-white scale-y-110"/>
                </Button>
            )}

            {state === 'idle' && buttonText && (
                <Button
                    size={buttonSize}
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className={`${containerClassFinal} gap-2 bg-blue-600 text-foreground font-medium`}
                >
                    <ArrowDown className="h-4 w-4 text-white scale-y-110 font-bold"/>
                    {buttonText}
                </Button>
            )}

            {state === 'downloading' && (
                <button
                    onClick={onCancelClick}
                    className={`${containerClassFinal} relative bg-blue-600 hover:bg-blue-700 overflow-hidden`}
                    title="Cancel download"
                >
                    <div
                        className="absolute top-0 left-0 bottom-0 bg-blue-800 transition-all duration-300 ease-out"
                        style={{width: `${progress * 100}%`}}
                    />
                    <div className="relative flex items-center justify-center h-full z-10">
                        <span className="text-xs font-medium text-white tabular-nums">
                            {Math.round(progress * 100)}%
                        </span>
                    </div>
                </button>
            )}

            {state === 'waiting' && (
                <div className={`${containerClassFinal} flex items-center justify-center bg-primary/10`}>
                    <span className="text-xs font-medium text-white">0%</span>
                </div>
            )}

            {state === 'completed' && (
                <div className={`${containerClassFinal} flex items-center justify-center bg-green-500/20 border border-green-500/50`}>
                    <Check className="h-4 w-4 text-green-600 dark:text-green-400"/>
                </div>
            )}
        </div>
    )
}
