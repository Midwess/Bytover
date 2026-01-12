import * as React from "react"
import {cn} from "@/lib/utils"
import {X} from "lucide-react"
import {noop} from "motion";

interface MultiEmailInputProps {
    onEmailsChange: (emails: string[]) => void
    placeholder?: string
    className?: string
    disabled?: boolean
    maxEmails?: number
}

function MultiEmailInput({
                             onEmailsChange = noop,
                             placeholder = "Enter email addresses...",
                             className,
                             disabled = false,
                             maxEmails,
                             ...props
                         }: MultiEmailInputProps) {
    const [emails, setEmails] = React.useState<string[]>([])
    const [inputValue, setInputValue] = React.useState("")
    const inputRef = React.useRef<HTMLInputElement>(null)

    // Notify parent of email changes via useEffect to avoid setState-during-render
    React.useEffect(() => {
        onEmailsChange(emails)
    }, [emails])

    const validateEmail = React.useCallback((email: string) => {
        return /^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(email.trim())
    }, [])

    const addEmail = React.useCallback(
        (email: string) => {
            const trimmed = email.trim()

            if (!trimmed) return
            if (!validateEmail(trimmed)) return
            if (emails.includes(trimmed)) return
            if (maxEmails && emails.length >= maxEmails) return

            setEmails(prev => [...prev, trimmed])

            setInputValue("")
        },
        [emails, maxEmails, validateEmail]
    )

    const removeEmail = React.useCallback(
        (emailToRemove: string) => {
            setEmails(prev => prev.filter(e => e !== emailToRemove))
        },
        []
    )

    const handleKeyDown = (e: React.KeyboardEvent<HTMLInputElement>) => {
        if (disabled) return

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

        if (value.includes(",")) {
            const parts = value.split(",")
            const completed = parts.slice(0, -1)
            const remaining = parts[parts.length - 1]

            completed.forEach(part => addEmail(part))
            setInputValue(remaining)
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
        if (!disabled) {
            inputRef.current?.focus()
        }
    }

    return (
        <div
            className={cn(
                "file:text-foreground placeholder:text-muted-foreground selection:bg-primary selection:text-primary-foreground dark:bg-input/30 border-input flex min-h-9 w-full min-w-0 rounded-lg border bg-transparent text-base shadow-xs transition-[color,box-shadow] outline-none file:inline-flex file:h-7 file:border-0 file:bg-transparent file:text-sm file:font-medium disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 md:text-sm",
                "flex-wrap gap-1 p-1 cursor-text",
                className
            )}
            onClick={handleContainerClick}
            data-no-scrollbar
        >
            {[...emails || []].map((email => (
                <span
                    key={email}
                    className="inline-flex items-center gap-1 bg-primary/10 text-primary px-2 py-1 rounded-lg text-sm font-medium"
                >
                    {email}
                    {!disabled && (
                        <button
                            type="button"
                            onClick={e => {
                                e.stopPropagation()
                                removeEmail(email)
                            }}
                            className="hover:bg-primary/20 rounded-sm p-0.5 transition-colors"
                        >
                            <X className="w-3 h-3"/>
                        </button>
                    )}
        </span>
            )))}

            <input
                ref={inputRef}
                type="email"
                value={inputValue}
                onChange={handleInputChange}
                onKeyDown={handleKeyDown}
                onBlur={handleBlur}
                disabled={disabled}
                placeholder={emails.length === 0 ? placeholder : ""}
                className="flex-1 min-w-[120px] bg-transparent border-0 outline-none px-2 py-1 placeholder:text-muted-foreground disabled:cursor-not-allowed"
                style={{minHeight: "28px"}}
            />
        </div>
    )
}

export {MultiEmailInput}
export type {MultiEmailInputProps}
