'use client'

import { Download, Check, Square } from 'lucide-react'
import { Button } from '@/components/ui/button'

interface DownloadButtonWithProgressProps {
    /** Progress value between 0 and 1 */
    progress?: number
    /** Whether the download is ready to start */
    isReady?: boolean
    /** Whether the download is completed */
    isCompleted?: boolean
    /** Whether download is in progress */
    isInProgress?: boolean
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
}

export default function DownloadButtonWithProgress({
    progress = 0,
    isReady = true,
    isCompleted = false,
    isInProgress = false,
    onDownloadClick = () => {},
    onCancelClick = () => {},
    size = 40,
    className = ''
}: DownloadButtonWithProgressProps) {
    // Determine button state
    const getButtonState = () => {
        if (isCompleted) return 'completed'
        if (isInProgress) return 'downloading'
        return 'idle'
    }

    const state = getButtonState()

    return (
        <div className={`relative ${className}`} style={{ width: size, height: size }}>
            {/* Idle State - Download Button */}
            {state === 'idle' && (
                <Button
                    size="icon"
                    variant="ghost"
                    onClick={onDownloadClick}
                    disabled={!isReady}
                    className="h-full w-full rounded-lg bg-primary/10 hover:bg-primary/20 border border-border transition-colors"
                    style={{ width: size, height: size }}
                >
                    <Download className="h-[50%] w-[50%] text-foreground" />
                </Button>
            )}

            {/* Downloading State - Progress Bar with Cancel */}
            {state === 'downloading' && (
                <button
                    onClick={onCancelClick}
                    className="relative flex flex-col items-center justify-center h-full w-full rounded-lg bg-primary/10 hover:bg-destructive/20 border border-border transition-colors group overflow-hidden"
                    style={{ width: size, height: size }}
                    title="Cancel download"
                >
                    {/* Progress Bar - Bottom to Top */}
                    <div
                        className="absolute bottom-0 left-0 right-0 bg-primary/30 transition-all duration-300 ease-out"
                        style={{ height: `${progress * 100}%` }}
                    />

                    {/* Center content - Percentage or Stop icon on hover */}
                    <div className="relative flex items-center justify-center z-10">
                        <span className="text-[10px] font-medium text-foreground group-hover:hidden tabular-nums">
                            {Math.round(progress * 100)}%
                        </span>
                        <Square className="h-[35%] w-[35%] text-destructive fill-destructive hidden group-hover:block absolute" />
                    </div>
                </button>
            )}

            {/* Completed State - Checkmark (persists) */}
            {state === 'completed' && (
                <button
                    className="flex items-center justify-center h-full w-full rounded-lg bg-green-500/20 border border-green-500/50 cursor-default"
                    style={{ width: size, height: size }}
                    disabled
                >
                    <Check className="h-[50%] w-[50%] text-green-600 dark:text-green-400" />
                </button>
            )}
        </div>
    )
}
