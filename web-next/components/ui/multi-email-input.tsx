import * as React from "react"
import { cn } from "@/lib/utils"
import { X } from "lucide-react"
import toast from "react-hot-toast";

interface MultiEmailInputProps {
    emails: string[]
    onEmailsChange: (emails: string[]) => void
    placeholder?: string
    className?: string
    disabled?: boolean
    maxEmails?: number
}

function MultiEmailInput({
    emails,
    onEmailsChange,
    placeholder = "Enter email addresses...",
    className,
    disabled = false,
    maxEmails,
    ...props
}: MultiEmailInputProps) {
    const [inputValue, setInputValue] = React.useState("")
    const inputRef = React.useRef<HTMLInputElement>(null)

    const validateEmail = (email: string): boolean => {
        const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/
        return emailRegex.test(email.trim())
    }

    const addEmail = (email: string) => {
        const trimmedEmail = email.trim()
        if (
            trimmedEmail &&
            validateEmail(trimmedEmail) &&
            !emails.includes(trimmedEmail) &&
            (!maxEmails || emails.length < maxEmails)
        ) {
            onEmailsChange([...emails, trimmedEmail])
            setInputValue("")
        }
        else {
            toast("Invalid email address or already exists");
        }
    }

    const removeEmail = (emailToRemove: string) => {
        onEmailsChange(emails.filter(email => email !== emailToRemove))
    }

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (e.key === "Enter" || e.key === "," || e.key === " ") {
            e.preventDefault()
            if (inputValue.trim()) {
                addEmail(inputValue)
            }
        } else if (e.key === "Backspace" && !inputValue && emails.length > 0) {
            removeEmail(emails[emails.length - 1])
        }
    }

    const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
        const value = e.target.value
        
        // Check if the input contains a comma
        if (value.includes(",")) {
            const parts = value.split(",")
            const emailsToAdd = parts.slice(0, -1) // All parts except the last one
            const remainingValue = parts[parts.length - 1] // The last part after the last comma
            
            // Add all complete emails
            emailsToAdd.forEach(email => {
                if (email.trim()) {
                    addEmail(email)
                }
            })
            
            setInputValue(remainingValue)
        } else {
            setInputValue(value)
        }
    }

    const handleBlur = () => {
        if (inputValue.trim()) {
            addEmail(inputValue)
        }
    }

    const handleContainerClick = () => {
        if (!disabled && inputRef.current) {
            inputRef.current.focus()
        }
    }

    return (
        <div
            className={cn(
                "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex min-h-9 w-full min-w-0 rounded-md border bg-transparent text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                "focus-within:border-ring focus-within:ring-ring/50 focus-within:ring-[3px]",
                "flex-wrap gap-1 p-1 cursor-text",
                disabled && "opacity-50 cursor-not-allowed",
                className
            )}
            onClick={handleContainerClick}
            {...props}
        >
            {emails.map((email, index) => (
                <span
                    key={`${email}-${index}`}
                    className="inline-flex items-center gap-1 bg-primary/10 text-primary px-2 py-1 rounded-md text-sm font-medium"
                >
                    {email}
                    {!disabled && (
                        <button
                            type="button"
                            onClick={(e) => {
                                e.stopPropagation()
                                removeEmail(email)
                            }}
                            className="hover:bg-primary/20 rounded-sm p-0.5 transition-colors"
                        >
                            <X className="w-3 h-3" />
                        </button>
                    )}
                </span>
            ))}
            <input
                ref={inputRef}
                type="email"
                value={inputValue}
                onChange={handleInputChange}
                onKeyDown={handleKeyDown}
                onFocus={() => {}}
                onBlur={handleBlur}
                disabled={disabled}
                placeholder={emails.length === 0 ? placeholder : ""}
                className="flex-1 min-w-[120px] bg-transparent border-0 outline-none px-2 py-1 placeholder:text-muted-foreground disabled:cursor-not-allowed"
                style={{ minHeight: "28px" }}
            />
        </div>
    )
}

export { MultiEmailInput }
export type { MultiEmailInputProps }