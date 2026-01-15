import * as React from "react"

import { cn } from "@/lib/utils"

export interface CardProps extends React.HTMLAttributes<HTMLDivElement> {
  shadowSize?: number;
}

const Card = React.forwardRef<HTMLDivElement, CardProps>(
  ({ className, style, shadowSize = 1, ...props }, ref) => {
    const shadowOpacity = {
      highlight: 0.05 * shadowSize,
      layer1: 0.12 * shadowSize,
      layer2: 0.08 * shadowSize,
      layer3: 0.06 * shadowSize,
      layer4: 0.04 * shadowSize,
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
          boxShadow: `0px 0.5px 0px 0px rgba(255, 255, 255, ${shadowOpacity.highlight}) inset, 0px 0.5px 1px 0px rgba(0, 0, 0, ${shadowOpacity.layer1}), 0px 2px 4px 0px rgba(0, 0, 0, ${shadowOpacity.layer2}), 0px 4px 8px -2px rgba(0, 0, 0, ${shadowOpacity.layer3}), 0px 8px 16px -4px rgba(0, 0, 0, ${shadowOpacity.layer4})`,
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
