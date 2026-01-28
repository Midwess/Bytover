import * as React from "react"

import { cn } from "@/lib/utils"

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  shadowSize?: number;
  disableInnerShadow?: boolean;
}

const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, style, shadowSize = 1, disableInnerShadow = false, children, ...props }, ref) => {
    const boxShadow = React.useMemo(() => {
      if (shadowSize === 0) return 'none';

      const inner = disableInnerShadow ? '' : [
        `inset 0 2px 4px 0 color-mix(in oklch, var(--muted-foreground) ${8 * shadowSize}%, transparent)`,
        `inset 0 -1px 3px 0 color-mix(in oklch, var(--card) ${15 * shadowSize}%, transparent)`,
        `inset 0 8px 16px -4px color-mix(in oklch, var(--muted-foreground) ${5 * shadowSize}%, transparent)`,
      ].join(', ');

      const outer = [
        `0px 0.5px 0px 0px rgba(255, 255, 255, ${0.05 * shadowSize})`,
        `0px 0.5px 1px 0px rgba(0, 0, 0, ${0.12 * shadowSize})`,
        `0px 2px 4px 0px rgba(0, 0, 0, ${0.08 * shadowSize})`,
        `0px 4px 8px -2px rgba(0, 0, 0, ${0.06 * shadowSize})`,
        `0px 8px 16px -4px rgba(0, 0, 0, ${0.04 * shadowSize})`,
      ].join(', ');

      return inner ? `${inner}, ${outer}` : outer;
    }, [shadowSize, disableInnerShadow]);

    return (
      <div
        data-tauri-drag-region
        ref={ref}
        className={cn(
          "rounded-2xl bg-card text-card-foreground transition-all border-2 duration-300 hover:scale-[1.008] p-[1px]",
          className
        )}
        style={{
          transform: 'translateZ(0.5px)',
          boxShadow,
          ...style,
        }}
        {...props}
      >
        {children}
      </div>
    );
  }
);
Card.displayName = "Card"

const CardHeader = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("flex flex-col space-y-1.5 p-6", className)}
    {...props}
  />
))
CardHeader.displayName = "CardHeader"

const CardTitle = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("font-semibold leading-none tracking-tight", className)}
    {...props}
  />
))
CardTitle.displayName = "CardTitle"

const CardDescription = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("text-sm text-muted-foreground", className)}
    {...props}
  />
))
CardDescription.displayName = "CardDescription"

const CardContent = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div ref={ref} className={cn("p-6 pt-0", className)} {...props} />
))
CardContent.displayName = "CardContent"

const CardFooter = React.forwardRef<
  HTMLDivElement,
  React.HTMLAttributes<HTMLDivElement>
>(({ className, ...props }, ref) => (
  <div
    ref={ref}
    className={cn("flex items-center p-6 pt-0", className)}
    {...props}
  />
))
CardFooter.displayName = "CardFooter"

export { Card, CardHeader, CardFooter, CardTitle, CardDescription, CardContent }
