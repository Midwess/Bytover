import { ReactElement, useEffect, useState, useRef } from "react";
import clsx from "clsx";

type Props = {
    center?: ReactElement;
    /** Progress value from 0 to 1 */
    progress: number;
    /** Whether the task is in progress */
    isInProgress?: boolean;
    /** Whether the task is completed */
    isCompleted?: boolean;
    /** Diameter of the circle in pixels */
    size?: number;
    /** Thickness of the progress ring */
    strokeWidth?: number;
    /** Color of the progress arc */
    color?: string;
    /** Background circle color */
    trackColor?: string;
    /** Animation duration in ms */
    duration?: number;
    onClick?: () => void;
};

export default function CircleProgress({
    progress,
    isInProgress = true,
    isCompleted = false,
    size = 80,
    strokeWidth = 2,
    duration = 100,
    onClick = () => { },
    center
}: Props) {
    const radius = (size - strokeWidth) / 2;
    const circumference = 2 * Math.PI * radius;

    const [displayProgress, setDisplayProgress] = useState(progress);
    const [showGreen, setShowGreen] = useState(false);
    const [showCheckmark, setShowCheckmark] = useState(false);
    const [isVisible, setIsVisible] = useState(isInProgress);
    const prevInProgressRef = useRef(isInProgress);

    // Update progress smoothly
    useEffect(() => {
        setDisplayProgress(progress);
    }, [progress]);

    // Detect when isInProgress changes from true to false
    useEffect(() => {
        const wasInProgress = prevInProgressRef.current;
        const isNowInProgress = isInProgress;

        if (wasInProgress && !isNowInProgress) {
            // Transition from in progress to not in progress - show checkmark
            setShowCheckmark(true);
            setShowGreen(true);

            // Hide after 1 second
            const timer = setTimeout(() => {
                setIsVisible(false);
                setShowCheckmark(false);
            }, 1000);

            return () => clearTimeout(timer);
        } else if (isNowInProgress) {
            // Back to in progress - reset states
            setIsVisible(true);
            setShowGreen(false);
            setShowCheckmark(false);
        } else if (!isNowInProgress && !showCheckmark) {
            // Not in progress and not showing checkmark - hide immediately
            setIsVisible(false);
            setDisplayProgress(0);
        }

        prevInProgressRef.current = isInProgress;
    }, [isInProgress, showCheckmark, progress]);

    // Handle completion animation (alternative trigger)
    useEffect(() => {
        if (isCompleted && progress >= 1 && !showCheckmark) {
            // Show green animation
            setShowGreen(true);
            setShowCheckmark(true);

            // Start fade out after green animation
            const timer = setTimeout(() => {
                setIsVisible(false);
                setShowCheckmark(false);
            }, 1000);

            return () => clearTimeout(timer);
        }
    }, [isCompleted, progress, showCheckmark]);

    // When showing checkmark, ensure progress is at 100%
    const finalProgress = showCheckmark ? 1 : displayProgress;
    const offset = circumference * (1 - finalProgress);

    return (
        <div
            className={clsx(
                "relative flex items-center justify-center transition-all duration-500",
                showGreen && "scale-110"
            )}
            style={{
                width: size,
                height: size,
                visibility: isVisible ? 'visible' : 'hidden',
                opacity: isVisible ? (showCheckmark ? 1 : (showGreen ? 0 : 1)) : 0,
                transition: showCheckmark
                    ? 'opacity 0.3s ease-in, transform 0.3s ease-in'
                    : (showGreen ? 'opacity 0.5s ease-out 0.5s, transform 0.5s ease-out' : 'opacity 0.3s ease-out')
            }}
        >
            <svg className="transform -rotate-90" width={size} height={size}>
                {/* Background circle */}
                <circle
                    className="stroke-foreground/90"
                    fill="transparent"
                    strokeWidth={strokeWidth}
                    r={radius}
                    cx={size / 2}
                    cy={size / 2}
                />
                {/* Progress arc */}
                <circle
                    className={clsx(
                        showGreen ? "stroke-greenSecondary" : "stroke-bluePrimary",
                        "transition-all duration-500"
                    )}
                    fill="transparent"
                    strokeWidth={strokeWidth}
                    strokeLinecap="round"
                    strokeDasharray={circumference}
                    strokeDashoffset={offset}
                    r={radius}
                    cx={size / 2}
                    cy={size / 2}
                    style={{
                        transition: `stroke-dashoffset ${duration}ms ease-in-out, stroke 0.5s ease-in-out`,
                    }}
                />
            </svg>

            {
                showCheckmark ? (
                    // Green checkmark icon
                    <div className="absolute flex items-center justify-center">
                        <svg
                            width={size * 0.5}
                            height={size * 0.5}
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            strokeWidth={3}
                            strokeLinecap="round"
                            strokeLinejoin="round"
                            className="text-greenSecondary animate-in zoom-in duration-300"
                        >
                            <polyline points="20 6 9 17 4 12"></polyline>
                        </svg>
                    </div>
                ) : center ? (
                    <div onClick={onClick} className={clsx(
                        "absolute flex items-center justify-center",
                        "cursor-pointer m-3",
                        showGreen ? "bg-greenSecondary" : "bg-bluePrimary",
                        "transition-colors duration-500"
                    )}>{center}</div>
                ) : (
                    <div
                        onClick={onClick}
                        className={clsx(
                            "absolute flex items-center justify-center",
                            "cursor-pointer m-3",
                            showGreen ? "bg-greenSecondary" : "bg-bluePrimary",
                            "transition-colors duration-500"
                        )}
                        style={{
                            width: size * 0.4,
                            height: size * 0.4,
                            borderRadius: "24%",
                        }}
                    />
                )
            }
        </div>
    );
}
