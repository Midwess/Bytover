import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import {invoke} from "@tauri-apps/api/core"
import {getVersion} from "@tauri-apps/api/app"
import {getCurrentWindow} from "@tauri-apps/api/window"
import {listen} from "@tauri-apps/api/event"
import {Button} from "@/components/ui/button"
import {
    Info,
    RefreshCw,
    LogOut,
    Loader2,
    Check,
    Settings,
    Download,
    User,
    ChevronRight,
    ExternalLink,
    Shield,
    Search,
    Sparkles
} from "lucide-react"
import {
    checkForUpdate,
    installUpdate,
    onUpdateProgress,
    onUpdateFinished
} from "@/lib/updater"
import {motion, AnimatePresence} from "motion/react"
import { openUrl } from "@tauri-apps/plugin-opener"

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
    is_critical: boolean
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

    useEffect(() => {
        const unlistenPromise = listen<string>("settings-set-tab", (event) => {
            const next = event.payload as SettingsTab
            if (next === "general" || next === "about" || next === "updates" || next === "account") {
                setActiveTab(next)
            }
        })
        return () => {
            unlistenPromise.then((unlisten) => unlisten())
        }
    }, [])

    const handleCheckUpdate = async () => {
        setIsCheckingUpdate(true)
        setUpdateStatus(null)
        try {
            const status = await checkForUpdate()
            setUpdateStatus(status)
        } catch (error) {
            console.error("Failed to check for update:", error)
            setUpdateStatus({available: false, version: null, release_notes: null, is_critical: false})
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

    const tabs: {id: SettingsTab; label: string; description: string; icon: React.ReactNode; color: string}[] = [
        {
            id: "general",
            label: "General",
            description: "Configure how Bytover starts up and behaves on your system.",
            icon: <Settings />,
            color: "bg-[#7c7c7c]"
        },
        {
            id: "account",
            label: "Account",
            description: "Manage your Bytover account and session.",
            icon: <User />,
            color: "bg-[#f39c12]"
        },
        {
            id: "updates",
            label: "Updates",
            description: "Keep your Bytover application up to date with the latest features.",
            icon: <RefreshCw />,
            color: "bg-[#3498db]"
        },
        {
            id: "about",
            label: "About",
            description: "Learn more about Bytover and its creators.",
            icon: <Info />,
            color: "bg-[#5856d6]"
        }
    ]

    const activeTabInfo = tabs.find(t => t.id === activeTab)

    return (
        <main className="w-screen h-screen dark bg-[#1e1e1e] text-white flex overflow-hidden font-sans select-none">
            {/* Sidebar */}
            <div 
                className="w-[180px] bg-[#262626] border-r border-black flex flex-col pt-12 pb-6 px-3 gap-1"
                data-tauri-drag-region
            >
                <div className="px-3 mb-4" data-tauri-drag-region>
                    <h1 className="text-[11px] font-medium text-white/30 uppercase tracking-wider">Settings</h1>
                </div>

                <div className="flex flex-col gap-0.5">
                    {tabs.map((tab) => (
                        <SidebarItem
                            key={tab.id}
                            icon={tab.icon}
                            iconColor={tab.color}
                            label={tab.label}
                            active={activeTab === tab.id}
                            onClick={() => setActiveTab(tab.id)}
                        />
                    ))}
                </div>

                <div className="mt-auto flex flex-col items-center gap-2 opacity-40">
                    <img src="/icon.png" alt="Bytover" className="w-8 h-8 rounded-lg" />
                    <div className="text-center">
                        <div className="text-[11px] font-medium text-white/90">Bytover</div>
                        <div className="text-xs text-white/40">Version {version}</div>
                    </div>
                </div>
            </div>

            {/* Content Area */}
            <div className="flex-1 flex flex-col bg-[#1e1e1e] overflow-y-auto" data-tauri-drag-region>
                <div className="w-full mx-auto px-6 py-12 flex flex-col gap-4">
                    {/* Header */}
                    <div className="flex flex-col items-center text-center gap-1 mb-1" data-tauri-drag-region>
                        <h2 className="text-[22px] font-bold tracking-tight text-white/95">
                            {activeTabInfo?.label}
                        </h2>
                        <p className="text-[12px] text-white/50 leading-snug max-w-[340px]">
                            {activeTabInfo?.description}
                        </p>
                    </div>
                    
                    <div className="flex-1">
                        <AnimatePresence mode="wait">
                            <motion.div
                                key={activeTab}
                                initial={{opacity: 0, y: 10}}
                                animate={{opacity: 1, y: 0}}
                                exit={{opacity: 0, y: -10}}
                                transition={{duration: 0.2, ease: "easeOut"}}
                            >
                                {activeTab === "general" && (
                                    <GeneralContent
                                        enabled={autoLaunchEnabled}
                                        isLoading={isLoadingAutoLaunch}
                                        onToggle={handleAutoLaunchToggle}
                                    />
                                )}
                                {activeTab === "account" && (
                                    <AccountContent onSignOut={handleSignOut} />
                                )}
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
                                {activeTab === "about" && (
                                    <AboutContent version={version} />
                                )}
                            </motion.div>
                        </AnimatePresence>
                    </div>
                </div>
            </div>
        </main>
    )
}

function SidebarItem({icon, iconColor, label, active, onClick}: {
    icon: React.ReactNode
    iconColor: string
    label: string
    active: boolean
    onClick: () => void
}) {
    return (
        <button
            onClick={onClick}
            className={`
                flex items-center gap-3 px-2 py-1.5 rounded-lg text-[13px] w-full text-left transition-all duration-200
                ${active 
                    ? "bg-white/10 text-white" 
                    : "text-white/80 hover:bg-white/5 hover:text-white"
                }
            `}
        >
            <div className={`w-[20px] h-[20px] rounded-[5px] ${iconColor} flex items-center justify-center shadow-inner shrink-0`}>
                {React.cloneElement(icon as React.ReactElement<any>, { className: "w-3 h-3 text-white" })}
            </div>
            <span className="font-medium tracking-tight">{label}</span>
        </button>
    )
}

function SettingsSection({title, children, description}: {
    title?: string
    children: React.ReactNode
    description?: string
}) {
    return (
        <div className="mb-4">
            {title && (
                <h3 className="text-[11px] font-semibold text-white/30 px-1 mb-1.5 uppercase tracking-wider">
                    {title}
                </h3>
            )}
            <div className="bg-[#2c2c2e] border border-white/5 rounded-xl overflow-hidden shadow-sm">
                {children}
            </div>
            {description && (
                <p className="mt-1.5 text-[11px] text-white/30 px-1 leading-relaxed">
                    {description}
                </p>
            )}
        </div>
    )
}

function SettingsRow({label, description, children, icon, last = false}: {
    label: string
    description?: string
    children: React.ReactNode
    icon?: React.ReactNode
    last?: boolean
}) {
    return (
        <div className={`
            flex items-center justify-between px-3.5 py-2.5
            ${!last ? "border-b border-white/5" : ""}
            hover:bg-white/[0.02] transition-colors
        `}>
            <div className="flex gap-3 items-start">
                {icon && <div className="mt-0.5 text-white/60">{icon}</div>}
                <div className="flex flex-col">
                    <span className="text-[13px] font-medium text-white/90">{label}</span>
                    {description && (
                        <span className="text-[11px] text-white/40 leading-tight">{description}</span>
                    )}
                </div>
            </div>
            <div className="flex items-center gap-3 shrink-0 ml-4">
                {children}
            </div>
        </div>
    )
}

function Switch({enabled, onToggle, disabled}: {
    enabled: boolean
    onToggle: (val: boolean) => void
    disabled?: boolean
}) {
    return (
        <button
            onClick={() => !disabled && onToggle(!enabled)}
            disabled={disabled}
            className={`
                w-[40px] h-[22px] rounded-full relative transition-all duration-300 ease-in-out
                ${enabled 
                    ? "bg-blue-500 shadow-[0_0_10px_rgba(59,130,246,0.2)]" 
                    : "bg-[#454545]"
                }
                ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-default"}
            `}
        >
            <motion.div
                animate={{x: enabled ? 20 : 2}}
                transition={{type: "spring", stiffness: 500, damping: 30}}
                className="absolute top-0.5 left-0 w-[18px] h-[18px] bg-white rounded-full shadow-lg"
            />
        </button>
    )
}

function GeneralContent({enabled, isLoading, onToggle}: {
    enabled: boolean
    isLoading: boolean
    onToggle: (enabled: boolean) => void
}) {
    return (
        <div className="space-y-6">
            <SettingsSection 
                title="Startup" 
            >
                <SettingsRow 
                    label="Open at Login" 
                    description="Automatically start Bytover when you log in."
                    last={true}
                >
                    <Switch enabled={enabled} onToggle={onToggle} disabled={isLoading} />
                </SettingsRow>
            </SettingsSection>

            <SettingsSection title="System">
                <SettingsRow 
                    label="Show in Menu Bar" 
                    description="Display Bytover icon in the system menu bar."
                    last={true}
                >
                    <Switch enabled={true} onToggle={() => {}} disabled={true} />
                </SettingsRow>
            </SettingsSection>
        </div>
    )
}

type PlanKind = "free" | "paid"

function PlanComparison({currentPlan, onUpgrade}: {currentPlan: PlanKind; onUpgrade: () => void}) {
    const rows: {label: string; free: string; paid: string}[] = [
        {label: "Files per transfer", free: "10", paid: "Unlimited"},
        {label: "Lifetime transfer", free: "5 GB", paid: "No cap"},
        {label: "Active shelves", free: "1", paid: "Unlimited"},
        {label: "Password-protected links", free: "—", paid: "Included"},
    ]

    return (
        <div className="px-4 py-3">
            <div className="grid grid-cols-[1fr_80px_80px] gap-x-4 pb-2 border-b border-white/5">
                <div />
                <div className="flex items-center justify-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-white/40">
                    Free
                    {currentPlan === "free" && (
                        <span className="px-1.5 py-[1px] rounded-full bg-amber-500/15 text-amber-300 text-[9px] font-bold tracking-wide">
                            YOU
                        </span>
                    )}
                </div>
                <div className="flex items-center justify-center gap-1.5 text-[10px] font-semibold uppercase tracking-wider text-white/40">
                    Paid
                    {currentPlan === "paid" && (
                        <span className="px-1.5 py-[1px] rounded-full bg-emerald-500/15 text-emerald-300 text-[9px] font-bold tracking-wide">
                            YOU
                        </span>
                    )}
                </div>
            </div>
            {rows.map((r, i) => (
                <div
                    key={i}
                    className={`grid grid-cols-[1fr_80px_80px] gap-x-4 py-2 text-[12px] items-center ${
                        i < rows.length - 1 ? "border-b border-white/[0.03]" : ""
                    }`}
                >
                    <div className="text-white/80 font-medium">{r.label}</div>
                    <div className="text-center text-white/40 tabular-nums">{r.free}</div>
                    <div className="text-center text-white/95 font-semibold tabular-nums">{r.paid}</div>
                </div>
            ))}
            {currentPlan === "free" && (
                <div className="pt-3 mt-3 border-t border-white/5 flex items-center justify-between">
                    <div className="flex flex-col">
                        <span className="text-[13px] font-semibold text-white">$20 · one-time</span>
                        <span className="text-[11px] text-white/40">Lifetime access. No subscription.</span>
                    </div>
                    <Button
                        size="sm"
                        onClick={onUpgrade}
                        className="h-[28px] px-4 text-[12px] bg-gradient-to-r from-amber-500 to-orange-500 hover:from-amber-400 hover:to-orange-400 text-white border-none rounded-full shadow-[0_0_12px_rgba(251,146,60,0.25)]"
                    >
                        <Sparkles className="w-3 h-3 mr-1" />
                        Upgrade
                    </Button>
                </div>
            )}
        </div>
    )
}

function AccountContent({onSignOut}: {onSignOut: () => void}) {
    const currentPlan: PlanKind = "free"
    const handleUpgrade = () => {}

    return (
        <div className="space-y-6">
            <SettingsSection title="Subscription">
                <PlanComparison currentPlan={currentPlan} onUpgrade={handleUpgrade} />
            </SettingsSection>

            <SettingsSection title="Current Session">
                <SettingsRow
                    label="Sign Out"
                    description="Disconnect your account and clear local data."
                    last={true}
                >
                    <Button
                        variant="secondary"
                        size="sm"
                        onClick={onSignOut}
                        className="h-[28px] px-4 text-[12px] bg-red-500/10 text-red-400 hover:bg-red-500/20 border-none rounded-full"
                    >
                        Sign Out
                    </Button>
                </SettingsRow>
            </SettingsSection>
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
        <div className="space-y-6">
            <SettingsSection title="Software Update">
                <SettingsRow 
                    label="Automatic Updates" 
                    description="Keep Bytover up to date automatically."
                    last={!status?.available}
                >
                    <Switch enabled={true} onToggle={() => {}} disabled={true} />
                </SettingsRow>
                
                {status?.available && (
                    <SettingsRow 
                        label="New Version Available" 
                        description={`Version ${status.version} is ready.`}
                        last={true}
                    >
                        {!isInstalling ? (
                            <Button
                                size="sm"
                                onClick={onInstall}
                                className="h-[28px] px-4 text-[12px] bg-blue-600 hover:bg-blue-500 text-white border-none rounded-full"
                            >
                                Update Now
                            </Button>
                        ) : (
                            <div className="text-[12px] font-medium text-blue-400">
                                {installProgress}%
                            </div>
                        )}
                    </SettingsRow>
                )}
            </SettingsSection>

            <div className="flex flex-col items-center justify-center py-6 gap-3">
                {isChecking ? (
                    <Loader2 className="w-5 h-5 animate-spin text-white/20" />
                ) : !status?.available ? (
                    <div className="flex flex-col items-center gap-2">
                        <div className="w-10 h-10 rounded-full bg-white/5 flex items-center justify-center">
                            <Check className="w-5 h-5 text-white/40" />
                        </div>
                        <span className="text-[13px] text-white/40">Your software is up to date</span>
                        <Button
                            variant="ghost"
                            size="sm"
                            onClick={onCheck}
                            className="text-[12px] text-blue-400 hover:text-blue-300 hover:bg-transparent"
                        >
                            Check for Updates
                        </Button>
                    </div>
                ) : null}
            </div>
        </div>
    )
}

function AboutContent({version}: {version: string}) {
    return (
        <div className="space-y-6">
            <SettingsSection>
                <button 
                    onClick={() => openUrl("https://bytover.com")}
                    className="w-full text-left"
                >
                    <SettingsRow 
                        label="Website" 
                        description="Visit bytover.com for more information."
                        icon={<ExternalLink className="w-4 h-4" />}
                    >
                        <ChevronRight className="w-4 h-4 text-white/20" />
                    </SettingsRow>
                </button>
                <button 
                    onClick={() => openUrl("https://bytover.com/policy")}
                    className="w-full text-left"
                >
                    <SettingsRow 
                        label="Privacy Policy" 
                        description="How we handle your data."
                        icon={<Shield className="w-4 h-4" />}
                        last={true}
                    >
                        <ChevronRight className="w-4 h-4 text-white/20" />
                    </SettingsRow>
                </button>
            </SettingsSection>

            <div className="flex flex-col items-center gap-1 mt-8 opacity-20">
                <span className="text-xs uppercase tracking-[0.2em] font-bold">Built with Tauri</span>
                <span className="text-xs">© 2026 Westrise</span>
            </div>
        </div>
    )
}

export default SettingsWindow
