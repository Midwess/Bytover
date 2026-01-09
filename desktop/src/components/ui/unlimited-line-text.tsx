import * as React from "react"
import { cn } from "@/lib/utils"

export interface UnlimitedLineTextProps
  extends React.HTMLAttributes<HTMLDivElement> {
  text: string
  /** Number of characters to show at the start when truncated */
  startChars?: number
  /** Number of characters to show at the end when truncated */
  endChars?: number
  /** Animation speed in pixels per second */
  speed?: number
  /** Delay in ms before animation starts on hover (default: 500) */
  hoverDelay?: number
}

const UnlimitedLineText = React.forwardRef<HTMLDivElement, UnlimitedLineTextProps>(
  ({
    className,
    text,
    startChars = 12,
    endChars = 12,
    speed = 50,
    hoverDelay = 500,
    ...props
  }, ref) => {
    const containerRef = React.useRef<HTMLDivElement>(null)
    const measureRef = React.useRef<HTMLSpanElement>(null)
    const hoverTimeoutRef = React.useRef<NodeJS.Timeout | null>(null)
    const [isOverflowing, setIsOverflowing] = React.useState(false)
    const [isHovered, setIsHovered] = React.useState(false)
    const [animationDuration, setAnimationDuration] = React.useState(5)

    // Check if text overflows the container
    React.useEffect(() => {
      const checkOverflow = () => {
        if (containerRef.current && measureRef.current) {
          const containerWidth = containerRef.current.offsetWidth
          const textWidth = measureRef.current.offsetWidth
          const overflow = textWidth > containerWidth
          setIsOverflowing(overflow)

          // Calculate animation duration based on text width and speed
          if (overflow) {
            const duration = textWidth / speed
            setAnimationDuration(Math.max(3, duration))
          }
        }
      }

      // Use requestAnimationFrame to ensure DOM is ready
      requestAnimationFrame(checkOverflow)

      const resizeObserver = new ResizeObserver(checkOverflow)
      if (containerRef.current) {
        resizeObserver.observe(containerRef.current)
      }

      return () => resizeObserver.disconnect()
    }, [text, speed])

    // Cleanup timeout on unmount
    React.useEffect(() => {
      return () => {
        if (hoverTimeoutRef.current) {
          clearTimeout(hoverTimeoutRef.current)
        }
      }
    }, [])

    // Create middle-truncated text
    const truncatedText = React.useMemo(() => {
      if (!isOverflowing || text.length <= startChars + endChars + 3) {
        return text
      }
      const start = text.slice(0, startChars)
      const end = text.slice(-endChars)
      return `${start}...${end}`
    }, [text, startChars, endChars, isOverflowing])

    return (
      <div
        ref={ref}
        className={cn(
          "relative overflow-hidden whitespace-nowrap",
          className
        )}
        onMouseEnter={() => {
          if (hoverTimeoutRef.current) {
            clearTimeout(hoverTimeoutRef.current)
          }
          hoverTimeoutRef.current = setTimeout(() => {
            setIsHovered(true)
          }, hoverDelay)
        }}
        onMouseLeave={() => {
          if (hoverTimeoutRef.current) {
            clearTimeout(hoverTimeoutRef.current)
            hoverTimeoutRef.current = null
          }
          setIsHovered(false)
        }}
        {...props}
      >
        {/* Hidden element to measure full text width */}
        <span
          ref={measureRef}
          className="absolute left-0 top-0 whitespace-nowrap opacity-0 pointer-events-none"
          aria-hidden="true"
        >
          {text}
        </span>

        {/* Container for the visible text */}
        <div ref={containerRef} className="overflow-hidden w-full">
          {isHovered && isOverflowing ? (
            // Marquee animation on hover - contained within parent
            <div className="overflow-hidden w-full">
              <div
                className="inline-flex animate-marquee whitespace-nowrap"
                style={{
                  animationDuration: `${animationDuration}s`,
                }}
              >
                <span className="inline-block pr-8">{text}</span>
                <span className="inline-block pr-8">{text}</span>
              </div>
            </div>
          ) : (
            // Truncated text when not hovered
            <span className="block">
              {truncatedText}
            </span>
          )}
        </div>
      </div>
    )
  }
)

UnlimitedLineText.displayName = "UnlimitedLineText"

export { UnlimitedLineText }
