import * as React from "react"
import {useState} from "react"
import {Eye, EyeOff} from "lucide-react"
import {cn} from "@/lib/utils"

export interface PasswordInputProps extends Omit<React.ComponentProps<"input">, "type"> {
}

const PasswordInput = React.forwardRef<HTMLInputElement, PasswordInputProps>(
    ({className, ...props}, ref) => {
        const [showPassword, setShowPassword] = useState(false)

        return (
            <div className="relative w-full">
                <input
                    type={showPassword ? "text" : "password"}
                    className={cn(
                        "flex h-9 w-full rounded-lg border border-input bg-transparent px-3 py-1 pr-10 text-base shadow-sm transition-colors file:border-0 file:bg-transparent file:text-sm file:font-medium file:text-foreground placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                        className
                    )}
                    ref={ref}
                    {...props}
                />
                <button
                    type="button"
                    onClick={() => setShowPassword(!showPassword)}
                    className="absolute right-2 top-1/2 -translate-y-1/2 pt-0.5 px-1 rounded-md transition-colors focus:outline-none focus:ring-1 focus:ring-ring hover:bg-muted"
                >
                    {showPassword ? (
                        <EyeOff className="h-4 w-4 text-muted-foreground"/>
                    ) : (
                        <Eye className="h-4 w-4 text-muted-foreground"/>
                    )}
                </button>
            </div>
        )
    }
)
PasswordInput.displayName = "PasswordInput"

export {PasswordInput}
