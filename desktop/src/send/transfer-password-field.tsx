import {PasswordInput} from "@/components/ui/password-input"
import {Lock} from "lucide-react"
import core from "@/core.ts"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"

type TransferPasswordFieldProps = {
    value: string
    onChange: (value: string) => void
    disabled?: boolean
    className?: string
    maxLength?: number
}

export function TransferPasswordField({value, onChange, disabled, className, maxLength}: TransferPasswordFieldProps) {
    const payment = core.usePayment()
    const allowed = payment?.capabilities?.transfer_limits?.password_encryption_allowed ?? true

    if (allowed) {
        return (
            <PasswordInput
                className={className}
                value={value}
                onChange={(e) => onChange(e.target.value)}
                maxLength={maxLength}
                placeholder="Password (Optional)"
                disabled={disabled}
            />
        )
    }

    const handleClick = () => {
        invoke("show_settings_with_tab", {tab: "account"}).then(noop)
    }

    return (
        <button
            type="button"
            onClick={handleClick}
            className={`${className ?? ""} flex items-center gap-1.5 px-2.5 text-left rounded-md hover:bg-secondary/70 transition-colors`}
        >
            <Lock className="h-3.5 w-3.5 text-blue-400 shrink-0" />
            <span className="text-xs font-medium text-blue-400 truncate">
                Upgrade to set password
            </span>
        </button>
    )
}
