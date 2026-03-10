import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import {invoke} from "@tauri-apps/api/core"
import {getVersion} from "@tauri-apps/api/app"
import {getCurrentWindow} from "@tauri-apps/api/window"
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
    Search
} from "lucide-react"
import {
    checkForUpdate,
    installUpdate,
    onUpdateProgress,
    onUpdateFinished
} from "@/lib/updater"
import {motion, AnimatePresence} from "motion/react"

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

    const tabs: {id: SettingsTab; label: string; icon: React.ReactNode; color: string}[] = [
        {
            id: "general",
            label: "General",
            icon: <Settings className="w-3.5 h-3.5 text-white" />,
            color: "bg-gray-500"
        },
        {
            id: "account",
            label: "Account",
            icon: <User className="w-3.5 h-3.5 text-white" />,
            color: "bg-blue-500"
        },
        {
            id: "updates",
            label: "Updates",
            icon: <RefreshCw className="w-3.5 h-3.5 text-white" />,
            color: "bg-purple-500"
        },
        {
            id: "about",
            label: "About",
            icon: <Info className="w-3.5 h-3.5 text-white" />,
            color: "bg-indigo-500"
        }
    ]

    return (
        <main className="w-screen h-screen dark bg-[#1e1e1e] text-white flex overflow-hidden font-sans select-none">
            {/* Sidebar */}
            <div className="w-[220px] bg-[#252525]/50 border-r border-white/5 flex flex-col pt-12 pb-4 px-3 gap-1">
                <div className="px-3 mb-4">
                    <h1 className="text-xl font-bold tracking-tight opacity-90">Settings</h1>
                </div>

                <div className="relative mb-4 px-2">
                    <Search className="absolute left-4 top-1/2 -translate-y-1/2 w-3.5 h-3.5 text-white/30" />
                    <input
                        type="text"
                        placeholder="Search"
                        className="w-full bg-white/5 border-none rounded-md py-1 pl-8 pr-2 text-xs focus:ring-1 focus:ring-blue-500/50 outline-none placeholder:text-white/20"
                    />
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

                <div className="mt-auto px-4 flex flex-col items-center gap-1 opacity-20">
                    <span className="text-[10px]">Bytover Desktop</span>
                    <span className="text-[10px]">v{version}</span>
                </div>
            </div>

            {/* Content Area */}
            <div className="flex-1 flex flex-col bg-[#1e1e1e]">
                <div className="h-12 flex items-center px-8 pt-8 pb-4">
                    <h2 className="text-xl font-semibold capitalize">{activeTab}</h2>
                </div>
                
                <div className="flex-1 overflow-y-auto px-8 py-4 custom-scrollbar">
                    <AnimatePresence mode="wait">
                        <motion.div
                            key={activeTab}
                            initial={{opacity: 0, y: 5}}
                            animate={{opacity: 1, y: 0}}
                            exit={{opacity: 0, y: -5}}
                            transition={{duration: 0.15, ease: "easeOut"}}
                            className="max-w-2xl mx-auto"
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
                flex items-center gap-3 px-2.5 py-1.5 rounded-lg text-[13px] w-full text-left transition-all duration-200
                ${active 
                    ? "bg-white/10 text-white shadow-sm" 
                    : "text-white/70 hover:bg-white/5 hover:text-white"
                }
            `}
        >
            <div className={`w-6 h-6 rounded-md ${iconColor} flex items-center justify-center shadow-inner`}>
                {icon}
            </div>
            <span className="font-medium">{label}</span>
            {active && (
                <div className="ml-auto w-1 h-3 bg-blue-500 rounded-full opacity-0" />
            )}
        </button>
    )
}

function SettingsSection({title, children, description}: {
    title?: string
    children: React.ReactNode
    description?: string
}) {
    return (
        <div className="mb-8">
            {title && (
                <h3 className="text-[13px] font-semibold text-white/40 px-1 mb-2 uppercase tracking-wider">
                    {title}
                </h3>
            )}
            <div className="bg-white/[0.03] border border-white/[0.05] rounded-xl overflow-hidden shadow-sm">
                {children}
            </div>
            {description && (
                <p className="mt-2 text-xs text-white/40 px-1 leading-relaxed">
                    {description}
                </p>
            )}
        </div>
    )
}

function SettingsRow({label, description, children, icon}: {
    label: string
    description?: string
    children: React.ReactNode
    icon?: React.ReactNode
}) {
    return (
        <div className="flex items-center justify-between px-4 py-3 border-b border-white/[0.05] last:border-none hover:bg-white/[0.01] transition-colors">
            <div className="flex flex-col gap-0.5 max-w-[70%]">
                <div className="flex items-center gap-2">
                    {icon && <div className="text-white/60">{icon}</div>}
                    <span className="text-[13px] font-medium text-white/90">{label}</span>
                </div>
                {description && (
                    <span className="text-xs text-white/40">{description}</span>
                )}
            </div>
            <div className="flex items-center gap-3">
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
                w-[38px] h-[22px] rounded-full relative transition-all duration-300 ease-in-out border
                ${enabled 
                    ? "bg-blue-500 border-blue-400 shadow-[0_0_10px_rgba(59,130,246,0.2)]" 
                    : "bg-white/10 border-white/5"
                }
                ${disabled ? "opacity-50 cursor-not-allowed" : "cursor-default"}
            `}
        >
            <motion.div
                animate={{x: enabled ? 16 : 0}}
                transition={{type: "spring", stiffness: 500, damping: 30}}
                className="absolute top-0.5 left-0.5 w-[18px] h-[18px] bg-white rounded-full shadow-lg"
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
        <div className="animate-in fade-in duration-300">
            <SettingsSection 
                title="Startup" 
                description="Managing how Bytover starts up can help improve your computer's performance and ensure you're always ready to share."
            >
                <SettingsRow 
                    label="Open at Login" 
                    description="Automatically start Bytover when you log in."
                >
                    <Switch enabled={enabled} onToggle={onToggle} disabled={isLoading} />
                </SettingsRow>
            </SettingsSection>
        </div>
    )
}

function AccountContent({onSignOut}: {onSignOut: () => void}) {
    return (
        <div className="animate-in fade-in duration-300">
            <SettingsSection title="Current Session">
                <SettingsRow 
                    label="Sign Out" 
                    description="Disconnect your account from this device. You will need to sign in again to use cloud features."
                >
                    <Button
                        variant="ghost"
                        size="sm"
                        onClick={onSignOut}
                        className="h-8 px-3 text-[12px] bg-red-500/10 text-red-400 hover:bg-red-500/20 border-none"
                    >
                        <LogOut className="w-3.5 h-3.5 mr-2" />
                        Sign Out
                    </Button>
                </SettingsRow>
            </SettingsSection>
            
            <SettingsSection title="Security">
                <SettingsRow label="Two-Factor Authentication" description="Add an extra layer of security to your account.">
                    <span className="text-[11px] text-white/30 italic">Coming soon</span>
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
        <div className="animate-in fade-in duration-300">
            <SettingsSection title="Software Update">
                <SettingsRow 
                    label="Automatic Updates" 
                    description="Keep Bytover up to date automatically."
                >
                    <Switch enabled={true} onToggle={() => {}} disabled={true} />
                </SettingsRow>
                
                <div className="px-4 py-6 flex flex-col items-center justify-center gap-4 bg-white/[0.01]">
                    {isChecking ? (
                        <div className="flex flex-col items-center gap-2">
                            <Loader2 className="w-6 h-6 animate-spin text-blue-500" />
                            <span className="text-[13px] text-white/60">Checking for updates...</span>
                        </div>
                    ) : status?.available ? (
                        <div className="flex flex-col items-center gap-3 w-full">
                            <div className="w-12 h-12 rounded-full bg-blue-500/20 flex items-center justify-center">
                                <Download className="w-6 h-6 text-blue-500" />
                            </div>
                            <div className="text-center">
                                <h4 className="text-[15px] font-semibold">New Update Available</h4>
                                <p className="text-xs text-white/40 mt-1">Version {status.version} is ready to install.</p>
                            </div>
                            {!isInstalling ? (
                                <Button
                                    onClick={onInstall}
                                    className="bg-blue-600 hover:bg-blue-500 text-white border-none px-6 rounded-full"
                                >
                                    Update Now
                                </Button>
                            ) : (
                                <div className="w-full max-w-xs flex flex-col gap-2 mt-2">
                                    <div className="flex justify-between text-[10px] text-white/40 uppercase tracking-widest font-bold">
                                        <span>Downloading</span>
                                        <span>{installProgress}%</span>
                                    </div>
                                    <div className="w-full h-1.5 bg-white/10 rounded-full overflow-hidden">
                                        <motion.div 
                                            initial={{width: 0}}
                                            animate={{width: `${installProgress}%`}}
                                            className="h-full bg-blue-500 shadow-[0_0_8px_rgba(59,130,246,0.5)]" 
                                        />
                                    </div>
                                </div>
                            )}
                        </div>
                    ) : (
                        <div className="flex flex-col items-center gap-3">
                            <div className="w-12 h-12 rounded-full bg-green-500/20 flex items-center justify-center">
                                <Check className="w-6 h-6 text-green-500" />
                            </div>
                            <div className="text-center">
                                <h4 className="text-[15px] font-semibold">Up to Date</h4>
                                <p className="text-xs text-white/40 mt-1">Bytover is running the latest version.</p>
                            </div>
                            <Button
                                variant="outline"
                                size="sm"
                                onClick={onCheck}
                                className="mt-2 rounded-full bg-white/5 border-white/10 hover:bg-white/10"
                            >
                                Check Again
                            </Button>
                        </div>
                    )}
                </div>
            </SettingsSection>
        </div>
    )
}

function AboutContent({version}: {version: string}) {
    return (
        <div className="flex flex-col items-center gap-8 py-4 animate-in fade-in slide-in-from-bottom-2 duration-500">
            <div className="relative group">
                <div className="absolute -inset-4 bg-blue-500/20 rounded-[32px] blur-xl group-hover:bg-blue-500/30 transition-all duration-500" />
                <img
                    src="/icon.png"
                    alt="Bytover"
                    className="w-24 h-24 rounded-2xl relative shadow-2xl grayscale-[0.2] group-hover:grayscale-0 transition-all duration-500"
                />
            </div>
            
            <div className="flex flex-col items-center gap-1.5">
                <h1 className="text-2xl font-bold tracking-tight bg-clip-text text-transparent bg-gradient-to-b from-white to-white/60">
                    Bytover
                </h1>
                <span className="text-xs font-mono text-white/40 bg-white/5 px-2 py-0.5 rounded-full border border-white/5">
                    Version {version}
                </span>
            </div>

            <p className="text-[13px] text-white/50 text-center max-w-[320px] leading-relaxed font-medium">
                Generate instant P2P links to share files directly with anyone. No uploads, no cloud, just peer-to-peer.
            </p>
            
            <div className="flex flex-col gap-3 w-full max-w-[300px] mt-2">
                <a 
                    href="#" 
                    className="flex items-center justify-between px-4 py-3 bg-white/5 hover:bg-white/10 rounded-xl transition-all group"
                >
                    <span className="text-[13px] font-medium text-white/80 group-hover:text-white">Website</span>
                    <ChevronRight className="w-3.5 h-3.5 text-white/20 group-hover:text-white/50" />
                </a>
                <a 
                    href="#" 
                    className="flex items-center justify-between px-4 py-3 bg-white/5 hover:bg-white/10 rounded-xl transition-all group"
                >
                    <span className="text-[13px] font-medium text-white/80 group-hover:text-white">Privacy Policy</span>
                    <ChevronRight className="w-3.5 h-3.5 text-white/20 group-hover:text-white/50" />
                </a>
            </div>

            <div className="flex flex-col items-center gap-1 mt-8">
                <span className="text-[10px] text-white/20 uppercase tracking-[0.2em] font-bold">Built with Tauri</span>
                <span className="text-[10px] text-white/10">© 2026 Westrise</span>
            </div>
        </div>
    )
}

export default SettingsWindow
