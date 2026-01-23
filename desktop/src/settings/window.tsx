import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import {invoke} from "@tauri-apps/api/core"
import {getVersion} from "@tauri-apps/api/app"
import {getCurrentWindow} from "@tauri-apps/api/window"
import {Button} from "@/components/ui/button"
import {Info, RefreshCw, LogOut, Loader2, Check} from "lucide-react"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <SettingsWindow/>
    </React.StrictMode>,
)

type SettingsTab = "about" | "updates" | "account"

interface UpdateStatus {
    available: boolean
    version: string | null
    release_notes: string | null
}

function SettingsWindow() {
    const [activeTab, setActiveTab] = useState<SettingsTab>("about")
    const [version, setVersion] = useState<string>("")
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false)
    const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null)

    useEffect(() => {
        getVersion().then(setVersion)
    }, [])

    const handleCheckUpdate = async () => {
        setIsCheckingUpdate(true)
        setUpdateStatus(null)
        try {
            const status = await invoke<UpdateStatus>("check_for_update")
            setUpdateStatus(status)
        } catch (error) {
            console.error("Failed to check for update:", error)
            setUpdateStatus({available: false, version: null, release_notes: null})
        } finally {
            setIsCheckingUpdate(false)
        }
    }

    const handleSignOut = async () => {
        await invoke("sign_out")
        getCurrentWindow()?.close()
    }

    return (
        <main className="w-screen h-screen dark bg-black/70 overflow-hidden">
            <div className="w-full h-full flex">
                <div className="w-[160px] bg-white/5 border-r border-white/10 flex flex-col pt-2 pb-2 px-2 gap-0.5">
                    <SidebarItem
                        icon={<Info className="w-4 h-4"/>}
                        label="About"
                        active={activeTab === "about"}
                        onClick={() => setActiveTab("about")}
                    />
                    <SidebarItem
                        icon={<RefreshCw className="w-4 h-4"/>}
                        label="Updates"
                        active={activeTab === "updates"}
                        onClick={() => setActiveTab("updates")}
                    />
                    <SidebarItem
                        icon={<LogOut className="w-4 h-4"/>}
                        label="Account"
                        active={activeTab === "account"}
                        onClick={() => setActiveTab("account")}
                    />
                </div>

                <div className="flex-1 flex flex-col">
                    <div className="flex-1 p-4 overflow-y-auto">
                        {activeTab === "about" && <AboutContent version={version}/>}
                        {activeTab === "updates" && (
                            <UpdatesContent
                                isChecking={isCheckingUpdate}
                                status={updateStatus}
                                onCheck={handleCheckUpdate}
                            />
                        )}
                        {activeTab === "account" && <AccountContent onSignOut={handleSignOut}/>}
                    </div>
                </div>
            </div>
        </main>
    )
}

function SidebarItem({icon, label, active, onClick}: {
    icon: React.ReactNode
    label: string
    active: boolean
    onClick: () => void
}) {
    return (
        <button
            onClick={onClick}
            className={`
                flex items-center gap-2 px-3 py-1.5 rounded-md text-sm w-full text-left transition-colors
                ${active ? "bg-white/15 text-white" : "text-white/70 hover:bg-white/10 hover:text-white"}
            `}
        >
            {icon}
            {label}
        </button>
    )
}

function AboutContent({version}: {version: string}) {
    return (
        <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-1">
                <span className="text-lg font-medium text-white">Bytover</span>
                <span className="text-sm text-white/60">Version {version}</span>
            </div>
            <p className="text-sm text-white/60">
                Transfer files between devices seamlessly using peer-to-peer connections.
            </p>
        </div>
    )
}

function UpdatesContent({isChecking, status, onCheck}: {
    isChecking: boolean
    status: UpdateStatus | null
    onCheck: () => void
}) {
    return (
        <div className="flex flex-col gap-4">
            <Button
                variant="outline"
                size="sm"
                onClick={onCheck}
                disabled={isChecking}
                className="w-fit gap-2 bg-white/10 border-white/20 text-white hover:bg-white/20"
            >
                {isChecking ? (
                    <>
                        <Loader2 className="w-4 h-4 animate-spin"/>
                        Checking...
                    </>
                ) : (
                    <>
                        <RefreshCw className="w-4 h-4"/>
                        Check for Updates
                    </>
                )}
            </Button>
            {status && (
                <div className="text-sm text-white/60">
                    {status.available ? (
                        <span>Update available: v{status.version}</span>
                    ) : (
                        <span className="flex items-center gap-1">
                            <Check className="w-4 h-4"/>
                            You're up to date
                        </span>
                    )}
                </div>
            )}
        </div>
    )
}

function AccountContent({onSignOut}: {onSignOut: () => void}) {
    return (
        <div className="flex flex-col gap-4">
            <p className="text-sm text-white/60">
                Sign out of your account on this device.
            </p>
            <Button
                variant="outline"
                size="sm"
                onClick={onSignOut}
                className="w-fit gap-2 bg-white/10 border-white/20 text-white hover:bg-white/20"
            >
                <LogOut className="w-4 h-4"/>
                Sign Out
            </Button>
        </div>
    )
}

export default SettingsWindow
