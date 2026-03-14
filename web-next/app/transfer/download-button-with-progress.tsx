'use client'

import {Download, Check, ArrowDown} from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useState, useEffect, useRef } from 'react'

interface DownloadButtonWithProgressProps {
    /** Progress value between 0 and 1 */
    progress?: number
    /** Whether the download is ready to start */
    isReady?: boolean
    /** Whether the download is completed */
    isCompleted?: boolean
    /** Whether download is in progress */
    isInProgress?: boolean
    /** Whether this is a cloud session (cannot cancel, shows waiting) */
    isCloud?: boolean
    /** Callback when download button is clicked */
    onDownloadClick?: () => void
    /** Callback when cancel is clicked during download */
    onCancelClick?: () => void
    /** Size of the button/progress indicator */
    size?: number
    /** Stroke width for the progress bar */
    strokeWidth?: number
    /** Custom class name */
    className?: string
    /** Text to display on button when idle. If provided, shows text button instead of icon button */
    buttonText?: string
    /** Button variant when idle */
    buttonVariant?: 'default' | 'outline' | 'ghost'
    /** Button size when idle */
    buttonSize?: 'default' | 'sm' | 'lg' | 'icon'
}

export default function DownloadButtonWithProgress({
    progress = 0,
    isReady = true,
    isCompleted = false,
    isInProgress = false,
    isCloud = false,
    onDownloadClick = () => {},
    onCancelClick = () => {},
    size = 40,
    className = '',
    buttonText,
    buttonVariant = 'outline',
    buttonSize = 'sm'
}: DownloadButtonWithProgressProps) {
    const [showCompleted, setShowCompleted] = useState(false)
    const wasInProgressRef = useRef(false)

    useEffect(() => {
        // Track if download was in progress
        if (isInProgress) {
            wasInProgressRef.current = true
        }

        // Only show green completed state if transitioning from in-progress to complete
        if (isCompleted && wasInProgressRef.current) {
            setShowCompleted(true)
            const timer = setTimeout(() => {
                setShowCompleted(false)
                wasInProgressRef.current = false
            }, 1000)
            return () => clearTimeout(timer)
        } else if (!isInProgress && !isCompleted) {
            // Reset when returning to idle state
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

    return (
        <div className={`relative ${className}`} style={buttonText ? {} : { width: size, height: size }}>
            {/* Idle State - Download Button */}
            {state === 'idle' && !buttonText && (
                <Button
                    size="icon"
                    variant="ghost"
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className="h-full w-full rounded-full bg-blue-600 hover:bg-blue-700 transition-colors"
                    style={{ width: size, height: size }}
                >
                    <ArrowDown className="h-[50%] w-[50%] text-white scale-y-110" />
                </Button>
            )}

            {/* Idle State - Text Button */}
            {state === 'idle' && buttonText && (
                <Button
                    variant={buttonVariant}
                    size={buttonSize}
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className="h-8 gap-2"
                >
                    <ArrowDown className="h-4 w-4 scale-y-110" />
                    {buttonText}
                </Button>
            )}

            {/* Downloading State - Progress Bar with Cancel */}
            {state === 'downloading' && (
                <button
                    onClick={onCancelClick}
                    className="relative flex flex-col items-center justify-center h-full w-full rounded-full bg-blue-600 hover:bg-blue-700 transition-colors group overflow-hidden"
                    style={{ width: size, height: size }}
                    title="Cancel download"
                >
                    {/* Progress Bar - Bottom to Top */}
                    <div
                        className="absolute bottom-0 left-0 right-0 bg-blue-800 transition-all duration-300 ease-out"
                        style={{ height: `${progress * 100}%` }}
                    />

                    {/* Center content - Percentage */}
                    <div className="relative flex items-center justify-center z-10">
                        <span className="text-[10px] font-medium text-white tabular-nums">
                            {Math.round(progress * 100)}%
                        </span>
                    </div>
                </button>
            )}

            {/* Waiting State - Cloud session waiting for download to start */}
            {state === 'waiting' && (
                <div
                    className="flex items-center justify-center h-full w-full rounded-full bg-primary/10 hover:bg-primary/20 transition-colors"
                    style={{ width: size, height: size }}
                >
                    <span className="text-[10px] font-medium text-white">0%</span>
                </div>
            )}

            {/* Completed State - Checkmark (persists) */}
            {state === 'completed' && (
                <button
                    className="flex items-center justify-center h-full w-full rounded-full bg-green-500/20 border border-green-500/50 cursor-default"
                    style={{ width: size, height: size }}
                    disabled
                >
                    <Check className="h-[50%] w-[50%] text-green-600 dark:text-green-400" />
                </button>
            )}
        </div>
    )
}
