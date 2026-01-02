'use client'

import { ArrowDown } from 'lucide-react'
import { useState, useEffect } from 'react'
import CircleProgress from '@/components/ui/progress'

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
    /** Stroke width for the progress circle */
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
    strokeWidth = 4,
    className = ''
}: DownloadButtonWithProgressProps) {
    const [showProgress, setShowProgress] = useState(false)

    // Determine if we should show progress indicator
    useEffect(() => {
        if (isInProgress || (progress > 0 && !isCompleted)) {
            setShowProgress(true)
        } else if (isCompleted || progress === 0) {
            // Delay hiding progress to allow smooth transition
            const timeout = setTimeout(() => setShowProgress(false), 300)
            return () => clearTimeout(timeout)
        }
    }, [isInProgress, progress, isCompleted])

    const containerSize = size + 8 // Add padding

    return (
        <div
            className={`relative flex items-center justify-center ${className}`}
            style={{ width: containerSize, height: containerSize }}
        >
            {/* Download Button */}
            <button
                className={`
                    absolute rounded-xl bg-white/10 hover:bg-white/20 border border-white/20
                    transition-all duration-500 ease-out hover:scale-110 shadow-lg
                    flex items-center justify-center
                    ${showProgress ? 'opacity-0 scale-50 pointer-events-none' : 'opacity-100 scale-100'}
                `}
                onClick={onDownloadClick}
                disabled={!isReady || showProgress}
                style={{
                    width: size,
                    height: size,
                    transitionProperty: 'opacity, transform, scale'
                }}
            >
                <ArrowDown
                    className="text-white"
                    style={{
                        width: size * 0.5,
                        height: size * 0.5
                    }}
                />
            </button>

            {/* Progress Circle */}
            <div
                className={`
                    absolute flex items-center justify-center
                    transition-all duration-500 ease-out
                    ${showProgress ? 'opacity-100 scale-100' : 'opacity-0 scale-50 pointer-events-none'}
                `}
                style={{
                    transitionProperty: 'opacity, transform, scale'
                }}
            >
                <CircleProgress
                    isCompleted={isCompleted}
                    isInProgress={isInProgress}
                    progress={progress}
                    size={size}
                    strokeWidth={strokeWidth}
                    onClick={onCancelClick}
                />
            </div>
        </div>
    )
}
