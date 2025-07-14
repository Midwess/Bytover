import * as React from "react"
import {cn} from "@/lib/utils"
import {Eye, EyeOff} from "lucide-react"

function Input({
    className,
    type,
    ...props
}: React.ComponentProps<"input">) {
    const [showPassword, setShowPassword] = React.useState(false)

    const isPasswordType = type === "password"
    const inputType = isPasswordType && showPassword ? "text" : type

    const toggleShowPassword = () => {
        setShowPassword((prev) => !prev)
    }

    // For password fields, wrap input + icon
    if (isPasswordType) {
        return (
            <div className="relative w-full">
                <input
                    type={inputType}
                    data-slot="input"
                    className={cn(
                        "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                        "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
                        "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
                        className
                    )}
                    {...props}
                />
                <button
                    type="button"
                    onClick={toggleShowPassword}
                    className="absolute right-2 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground focus:outline-none"
                    tabIndex={-1}
                >
                    {showPassword ? (
                        <EyeOff className="w-4 h-4"/>
                    ) : (
                        <Eye className="w-4 h-4"/>
                    )}
                </button>
            </div>
        )
    }

    return (
        <input
            type={type}
            data-slot="input"
            className={cn(
                "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex h-9 w-full min-w-0 rounded-md border bg-transparent px-3 py-1 text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                "focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:ring-[3px]",
                "aria-invalid:ring-destructive/20 dark:aria-invalid:ring-destructive/40 aria-invalid:border-destructive",
                className
            )}
            {...props}
        />
    )
}

export {Input}