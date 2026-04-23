import {Button} from "@/components/ui/button"
import {Card} from "@/components/ui/card"
import {TransferPasswordField} from "@/send/transfer-password-field"
import {MultiEmailInput} from "@/components/ui/multi-email-input"
import core from "@/core.ts"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"
import {useState} from "react"
import {Progress} from "@/components/animate-ui/components/radix/progress"
import {ProgressIndicator} from "@/components/animate-ui/primitives/radix/progress"
import {Send} from "lucide-react"

export function EmailTransfer({ shelfId }: { shelfId: string | undefined }) {
    const [pwd, setPwd] = useState("");
    const [emails, setEmails] = useState<string[]>([]);
    const [isLoading, setIsLoading] = useState(false)
    const selectedResources = core.useSelectedResourcesForShelf(shelfId)
    const cloudSession = core.useCloudSessionForShelf(shelfId, true)
    const progress = (cloudSession?.progress ?? 0) * 100

    const handleEmailTransfer = () => {
        if (!shelfId || emails.length === 0 || isLoading) return
        if (selectedResources.length > 0) {
            setIsLoading(true)
            const delay = Math.random() * 2000 + 2000
            setTimeout(() => setIsLoading(false), delay)
        }
        invoke("email_transfer", {
            shelfId,
            password: pwd || null,
            toEmails: emails
        }).then(noop)
    }

    return <div className="flex flex-col gap-2 w-full overflow-hidden">
        <Card shadowSize={0.5} className="flex flex-col p-1 w-full">
            <MultiEmailInput
                onEmailsChange={(emails) => {
                    setEmails(emails)
                }}
                placeholder="Enter recipient emails"
                className="min-h-8 bg-secondary shadow-background max-h-[70px] overflow-y-auto w-full"
                disabled={!!cloudSession?.is_in_progress && !!cloudSession?.is_email}
            />
        </Card>
        <Card shadowSize={0.5} className="flex flex-col p-1 w-full">
            <TransferPasswordField
                className={"h-9 bg-secondary shadow-background w-full"}
                value={pwd}
                onChange={setPwd}
                disabled={!!cloudSession?.is_in_progress && !!cloudSession?.is_email}
            />
        </Card>
        <Card shadowSize={0.5} className={`flex flex-row gap-2 p-1 items-center ${cloudSession?.progress ? "w-full" : "w-fit"}`}>
            {
                cloudSession?.is_in_progress ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }} className={"bg-muted-foreground/30 text-primary w-[100px] h-full shadow-lg"}>Cancel</Button>
                ) : cloudSession?.is_completed ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }}
                            className={"bg-greenSecondary/40 text-primary flex-2/5 shadow-lg hover:bg-greenSecondary/50"}>Continue</Button>
                ) : (
                    <Button
                        onClick={handleEmailTransfer}
                        disabled={isLoading}
                        className={"bg-bluePrimary text-foreground w-[100px] shadow-lg hover:bg-bluePrimary/60 disabled:opacity-70"}>
                        {isLoading ? (
                            <div className="h-4 w-4 animate-spin rounded-full border-2 border-white/20 border-t-white"></div>
                        ) : (
                            <>Send <Send/></>
                        )}
                    </Button>
                )
            }
            {
                !!cloudSession?.progress && (
                    <div className="flex flex-col gap-2 pb-2 flex-3/5">
                        <div className="flex items-center justify-between gap-1">
                            <span className="text-sm">
                                {cloudSession?.display_download_speed}
                            </span>
                        </div>
                        <Progress value={progress} className="w-full space-y-2">
                            <ProgressIndicator className="bg-primary rounded-full h-full w-full flex-1"/>
                        </Progress>
                    </div>
                )
            }
        </Card>
    </div>
}