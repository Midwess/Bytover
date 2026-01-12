import {Button} from "@/components/ui/button"
import {Card} from "@/components/ui/card"
import {PasswordInput} from "@/components/ui/password-input"
import {MultiEmailInput} from "@/components/ui/multi-email-input"
import core from "@/core.ts"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"
import {useState} from "react"
import {Progress} from "@/components/animate-ui/components/radix/progress"
import {ProgressIndicator} from "@/components/animate-ui/primitives/radix/progress"

export function EmailTransfer({ shelfId }: { shelfId: string | undefined }) {
    const [pwd, setPwd] = useState("");
    const [emails, setEmails] = useState<string[]>([]);
    const cloudSession = core.useCloudSessionForShelf(shelfId)
    const progress = (cloudSession?.progress ?? 0) * 100

    const handleEmailTransfer = () => {
        if (!shelfId || emails.length === 0) return
        invoke("email_transfer", {
            shelfId,
            password: pwd || null,
            toEmails: emails
        }).then(noop)
    }

    return <>
        <Card shadowSize={0} className="flex flex-col p-1 w-[200px]">
            <MultiEmailInput
                onEmailsChange={(emails) => {
                    setEmails(emails)
                }}
                placeholder="Enter recipient emails"
                className="min-h-11 bg-secondary shadow-background max-h-[90px] overflow-y-auto"
                disabled={cloudSession?.is_in_progress ?? false}
            />
        </Card>
        <Card shadowSize={0} className="flex flex-col p-1">
            <PasswordInput
                className={"h-11 bg-secondary shadow-background"}
                value={pwd}
                onChange={(e) => {
                    setPwd(e.target.value)
                }}
                placeholder={"Password (Optional)"}
                disabled={cloudSession?.is_in_progress ?? false}
            />
        </Card>
        <Card className="flex flex-row gap-2 p-1 items-center">
            {
                cloudSession?.is_in_progress ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }} className={"bg-muted-foreground/30 text-primary w-[70px] h-full shadow-lg"}>Cancel</Button>
                ) : cloudSession?.is_completed ? (
                    <Button onClick={() => {
                        invoke("cancel_send", {sessionId: cloudSession?.session_id}).then(noop)
                    }}
                            className={"bg-greenSecondary/40 text-primary w-[70px] shadow-lg hover:bg-greenSecondary/50"}>Continue</Button>
                ) : (
                    <Button
                        onClick={handleEmailTransfer}
                        className={"bg-bluePrimary text-foreground w-[70px] shadow-lg hover:bg-bluePrimary/60 disabled:opacity-50"}>
                        Send
                    </Button>
                )
            }
            {
                !!cloudSession?.progress && (
                    <div className="flex flex-col w-full gap-2 pb-2 flex-1/2">
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
    </>
}