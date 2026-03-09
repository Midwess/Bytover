import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import {invoke} from "@tauri-apps/api/core"
import {getVersion} from "@tauri-apps/api/app"
import {getCurrentWindow} from "@tauri-apps/api/window"
import {Button} from "@/components/ui/button"
import {Info, RefreshCw, LogOut, Loader2, Check, Settings, Download, CircleCheck} from "lucide-react"
import {checkForUpdate, installUpdate, onUpdateProgress, onUpdateFinished, UpdateStatus as UpdaterUpdateStatus} from "@/lib/updater"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <SettingsWindow/>
    </React.StrictMode>,
)

type SettingsTab = "general" | "about" | "updates" | "account"

interface UpdateStatus {
    available: boolean
    version: string | null
    release_notes: string | null
}

function SettingsWindow() {
    const [activeTab, setActiveTab] = useState<SettingsTab>(() => {
        const params = new URLSearchParams(window.location.search)
        const tab = params.get("tab")
        return (tab as SettingsTab) || "general"
    })
    const [version, setVersion] = useState<string>("")
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false)
    const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null)
    const [isInstalling, setIsInstalling] = useState(false)
    const [installProgress, setInstallProgress] = useState(0)
    const [autoLaunchEnabled, setAutoLaunchEnabled] = useState(false)
    const [isLoadingAutoLaunch, setIsLoadingAutoLaunch] = useState(true)

    useEffect(() => {
        getVersion().then(setVersion)
    }, [])

    useEffect(() => {
        checkForUpdate()
            .then(setUpdateStatus)
            .catch(console.error)
    }, [])

    useEffect(() => {
        invoke<boolean>("is_autostart_enabled")
            .then(setAutoLaunchEnabled)
            .catch(console.error)
            .finally(() => setIsLoadingAutoLaunch(false))
    }, [])

    const handleCheckUpdate = async () => {
        setIsCheckingUpdate(true)
        setUpdateStatus(null)
        try {
            const status = await checkForUpdate()
            setUpdateStatus(status)
        } catch (error) {
            console.error("Failed to check for update:", error)
            setUpdateStatus({available: false, version: null, release_notes: null})
        } finally {
            setIsCheckingUpdate(false)
        }
    }

    const handleInstallUpdate = async () => {
        setIsInstalling(true)
        setInstallProgress(0)
        try {
            const unlistenProgress = await onUpdateProgress((progress) => {
                if (progress.total > 0) {
                    setInstallProgress(Math.round((progress.downloaded / progress.total) * 100))
                }
            })
            const unlistenFinished = await onUpdateFinished(() => {
                setIsInstalling(false)
            })
            await installUpdate()
            unlistenProgress()
            unlistenFinished()
        } catch (error) {
            console.error("Failed to install update:", error)
            setIsInstalling(false)
        }
    }

    const handleSignOut = async () => {
        await invoke("sign_out")
        getCurrentWindow()?.close()
    }

    const handleAutoLaunchToggle = async (enabled: boolean) => {
        setIsLoadingAutoLaunch(true)
        try {
            await invoke("set_autostart", {enabled})
            setAutoLaunchEnabled(enabled)
        } catch (error) {
            console.error("Failed to set autostart:", error)
        } finally {
            setIsLoadingAutoLaunch(false)
        }
    }

    return (
        <main className="w-screen h-screen dark bg-black/70 overflow-hidden">
            <div className="w-full h-full flex">
                <div className="w-[160px] bg-white/5 border-r border-white/10 flex flex-col pt-2 pb-2 px-2 gap-0.5">
                    <SidebarItem
                        icon={<Settings className="w-4 h-4"/>}
                        label="General"
                        active={activeTab === "general"}
                        onClick={() => setActiveTab("general")}
                    />
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
                        {activeTab === "general" && (
                            <GeneralContent
                                enabled={autoLaunchEnabled}
                                isLoading={isLoadingAutoLaunch}
                                onToggle={handleAutoLaunchToggle}
                            />
                        )}
                        {activeTab === "about" && <AboutContent version={version}/>}
                        {activeTab === "updates" && (
                            <UpdatesContent
                                isChecking={isCheckingUpdate}
                                status={updateStatus}
                                onCheck={handleCheckUpdate}
                                isInstalling={isInstalling}
                                installProgress={installProgress}
                                onInstall={handleInstallUpdate}
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
        <div className="flex flex-col items-center gap-6 py-8">
            <img
                src="/icon.png"
                alt="Bytover"
                className="w-24 h-24 rounded-2xl"
            />
            <div className="flex flex-col items-center gap-1">
                <span className="text-xl font-semibold text-white">Bytover</span>
                <span className="text-sm text-white/60">Version {version}</span>
            </div>
            <p className="text-sm text-white/60 text-center max-w-[280px]">
                Generate instant P2P links to share files directly with anyone. No uploads, no cloud, just peer-to-peer.
            </p>
            <div className="flex gap-4 mt-2">
                <span className="text-xs text-white/40">Built with Tauri</span>
            </div>
        </div>
    )
}

function UpdatesContent({isChecking, status, onCheck, isInstalling, installProgress, onInstall}: {
    isChecking: boolean
    status: UpdateStatus | null
    onCheck: () => void
    isInstalling: boolean
    installProgress: number
    onInstall: () => void
}) {
    return (
        <div className="flex flex-col gap-4">
            <Button
                variant="outline"
                size="sm"
                onClick={onCheck}
                disabled={isChecking || isInstalling}
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
                        <div className="flex flex-col gap-2">
                            <span>Update available: v{status.version}</span>
                            {!isInstalling && (
                                <Button
                                    variant="outline"
                                    size="sm"
                                    onClick={onInstall}
                                    className="w-fit gap-2 bg-white/10 border-white/20 text-white hover:bg-white/20"
                                >
                                    <Download className="w-4 h-4"/>
                                    Install Update
                                </Button>
                            )}
                        </div>
                    ) : (
                        <span className="flex items-center gap-1">
                            <Check className="w-4 h-4"/>
                            You're up to date
                        </span>
                    )}
                </div>
            )}
            {isInstalling && (
                <div className="flex flex-col gap-2">
                    <div className="flex items-center gap-2 text-sm text-white/60">
                        <Loader2 className="w-4 h-4 animate-spin"/>
                        Downloading update... {installProgress}%
                    </div>
                    <div className="w-full h-2 bg-white/10 rounded-full overflow-hidden">
                        <div
                            className="h-full bg-green-500 transition-all duration-300"
                            style={{width: `${installProgress}%`}}
                        />
                    </div>
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

function GeneralContent({enabled, isLoading, onToggle}: {
    enabled: boolean
    isLoading: boolean
    onToggle: (enabled: boolean) => void
}) {
    return (
        <div className="flex flex-col gap-4">
            <div className="flex items-center justify-between">
                <span className="text-sm text-white">Open at Login</span>
                <button
                    onClick={() => onToggle(!enabled)}
                    disabled={isLoading}
                    className={`w-10 h-5 rounded-full relative transition-colors ${enabled ? "bg-green-500" : "bg-white/20"}`}
                >
                    <span className={`absolute top-0.5 left-0.5 w-4 h-4 bg-white rounded-full shadow-md transition-transform ${enabled ? "translate-x-5" : "translate-x-0"}`} />
                </button>
            </div>
            <p className="text-sm text-white/60">
                Automatically start bit-bridge when you log in to your computer.
            </p>
        </div>
    )
}

export default SettingsWindow
