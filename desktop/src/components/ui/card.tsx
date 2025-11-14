import * as React from "react"

import { cn } from "@/lib/utils"

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  shadowSize?: number;
}

const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, style, shadowSize = 1, ...props }, ref) => {
    const shadowOpacity = {
      inset: 0.02 * shadowSize,
      shadow1: 0.15 * shadowSize,
      shadow2: 0.16 * shadowSize,
      shadow3: 0.16 * shadowSize,
      shadow4: 0.19 * shadowSize,
    };

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
          boxShadow: `1px 1px 16px 0px rgba(253, 253, 253, ${shadowOpacity.inset}) inset, 0px 26px 48px -10px rgba(8, 8, 8, ${shadowOpacity.shadow1}), 0px 12px 28px 0px rgba(8, 8, 8, ${shadowOpacity.shadow2}), 0px 4px 8px -2px rgba(8, 8, 8, ${shadowOpacity.shadow3}), 0px 4px 2px -1px rgba(0, 0, 0, ${shadowOpacity.shadow4})`,
          ...style,
        }}
        {...props}
      />
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
