"use client"

import * as React from "react"
import { cn } from "@/lib/utils"

export const DEFAULT_WORDS = [
    "Connecting...",
    "Negotiating...",
    "Establishing...",
]

interface SignallingAnimationProps {
    className?: string
    words?: string[]
    interval?: number
}

function SignallingAnimation({
    className,
    words = DEFAULT_WORDS,
    interval = 3000,
}: SignallingAnimationProps) {
    const [currentIndex, setCurrentIndex] = React.useState(0)
    const [isVisible, setIsVisible] = React.useState(true)

    React.useEffect(() => {
        const timer = setInterval(() => {
            setIsVisible(false)
            setTimeout(() => {
                setCurrentIndex((prev) => (prev + 1) % words.length)
                setIsVisible(true)
            }, 300)
        }, interval)

        return () => clearInterval(timer)
    }, [words.length, interval])

    return (
        <span
            className={cn(
                "inline-block transition-opacity duration-300",
                isVisible ? "opacity-100" : "opacity-0",
                className
            )}
        >
            {words[currentIndex]}
        </span>
    )
}

export { SignallingAnimation }
