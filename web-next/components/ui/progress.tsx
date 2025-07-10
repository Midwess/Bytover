import React from "react";
import clsx from "clsx";

type Props = {
    /** Progress value from 0 to 1 */
    progress: number;
    /** Diameter of the circle in pixels */
    size?: number;
    /** Thickness of the progress ring */
    strokeWidth?: number;
    /** Color of the progress arc */
    color?: string;
    /** Background circle color */
    trackColor?: string;
};

export default function CircleProgress({
    progress,
    size = 80,
    strokeWidth = 4,
}: Props) {
    const radius = (size - strokeWidth) / 2;
    const circumference = 2 * Math.PI * radius;
    const offset = circumference * (1 - progress);

    return (
        <div
            className="relative flex items-center justify-center"
            style={{width: size, height: size}}
        >
            {/* SVG Circle Progress */}
            <svg
                className="transform -rotate-90"
                width={size}
                height={size}
            >
                {/* Background circle */}
                <circle
                    className={"stroke-bluePrimary"}
                    fill="transparent"
                    strokeWidth={strokeWidth}
                    r={radius}
                    cx={size / 2}
                    cy={size / 2}
                />
                {/* Progress arc */}
                <circle
                    className={"stroke-gray-400"}
                    fill="transparent"
                    strokeWidth={strokeWidth}
                    strokeLinecap="round"
                    strokeDasharray={circumference}
                    strokeDashoffset={offset}
                    r={radius}
                    cx={size / 2}
                    cy={size / 2}
                />
            </svg>

            <div
                className={clsx(
                    "absolute flex items-center justify-center",
                    "cursor-pointer bg-bluePrimary"
                )}
                style={{
                    width: size * 0.4,
                    height: size * 0.4,
                    borderRadius: '24%'
                }}
            >
            </div>
        </div>
    );
}