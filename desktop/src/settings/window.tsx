import ReactDOM from "react-dom/client"
import React, {useEffect, useState} from "react"
import {invoke} from "@tauri-apps/api/core"
import {getVersion} from "@tauri-apps/api/app"
import {getCurrentWindow} from "@tauri-apps/api/window"
import {listen} from "@tauri-apps/api/event"
import {Button} from "@/components/ui/button"
import {
    RefreshCw,
    Loader2,
    Check,
    ChevronRight,
    ExternalLink,
    Shield,
    MessageSquare,
    Settings,
    User,
    Download,
    Info,
    Github,
} from "lucide-react"
import {
    checkForUpdate,
} from "@/lib/updater"
import {motion, AnimatePresence} from "motion/react"
import { openUrl } from "@tauri-apps/plugin-opener"
import core from "@/core.ts"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <SettingsWindow/>
    </React.StrictMode>,
)

type SettingsTab = "general" | "about" | "updates" | "account"

const IS_MACOS = typeof navigator !== "undefined" && /Mac/i.test(navigator.userAgent)

function isValidTab(value: string | null): value is SettingsTab {
    if (value === "general" || value === "about" || value === "account") return true
    if (value === "updates") return !IS_MACOS
    return false
}

interface UpdateStatus {
    available: boolean
    version: string | null
    release_notes: string | null
    is_critical: boolean
    store_url: string | null
}

function SettingsWindow() {
    const [activeTab, setActiveTab] = useState<SettingsTab>(() => {
        const params = new URLSearchParams(window.location.search)
        const tab = params.get("tab")
        return isValidTab(tab) ? tab : "general"
    })
    const [version, setVersion] = useState<string>("")
    const [isCheckingUpdate, setIsCheckingUpdate] = useState(false)
    const [updateStatus, setUpdateStatus] = useState<UpdateStatus | null>(null)
    const [autoLaunchEnabled, setAutoLaunchEnabled] = useState(false)
    const [isLoadingAutoLaunch, setIsLoadingAutoLaunch] = useState(true)

    useEffect(() => {
        core.launch()
    }, [])

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
            if (isValidTab(event.payload)) {
                setActiveTab(event.payload)
            }
        })
        return () => {
            unlistenPromise.then((unlisten) => unlisten())
        }
    }, [])

    useEffect(() => {
        if (IS_MACOS) return
        const unlistenPromise = listen("tray-update-clicked", () => {
            handleInstallUpdate()
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
            setUpdateStatus({available: false, version: null, release_notes: null, is_critical: false, store_url: null})
        } finally {
            setIsCheckingUpdate(false)
        }
    }

    const handleInstallUpdate = async () => {
        if (updateStatus?.store_url) {
            try {
                await openUrl(updateStatus.store_url)
            } catch (error) {
                console.error("Failed to open store URL:", error)
            }
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

    const tabs: {id: SettingsTab; label: string; description: string; icon: React.ReactNode}[] = [
        {
            id: "general",
            label: "General",
            description: "Configure how Bytover starts up and behaves on your system.",
            icon: <Settings className="w-[18px] h-[18px]" strokeWidth={1.75} />,
        },
        {
            id: "account",
            label: "Account",
            description: "Manage your Bytover account and session.",
            icon: <User className="w-[18px] h-[18px]" strokeWidth={1.75} />,
        },
        ...(IS_MACOS ? [] : [{
            id: "updates" as const,
            label: "Updates",
            description: "Keep your Bytover application up to date with the latest features.",
            icon: <Download className="w-[18px] h-[18px]" strokeWidth={1.75} />,
        }]),
        {
            id: "about",
            label: "About",
            description: "Learn more about Bytover and its creators.",
            icon: <Info className="w-[18px] h-[18px]" strokeWidth={1.75} />,
        },
    ]

    const activeTabInfo = tabs.find(t => t.id === activeTab)

    return (
        <main
            className="w-screen h-screen dark text-white flex overflow-hidden font-sans select-none"
            style={{background: "#090e1b"}}
        >
            {/* Sidebar */}
            <div
                className="w-[80px] flex flex-col pt-10 pb-5 px-2 shrink-0"
                data-tauri-drag-region
                style={{background: "#090e1b"}}
            >
                <div className="flex flex-col gap-1">
                    {tabs.map((tab) => (
                        <SidebarItem
                            key={tab.id}
                            label={tab.label}
                            icon={tab.icon}
                            active={activeTab === tab.id}
                            onClick={() => setActiveTab(tab.id)}
                        />
                    ))}
                </div>

                <div className="mt-auto flex justify-center">
                    <SidebarProfile />
                </div>
            </div>

            {/* Content Area */}
            <div
                className="flex-1 flex flex-col overflow-y-auto rounded-2xl m-2 relative"
                data-tauri-drag-region
                style={{background: "linear-gradient(to bottom, #0c1c4d 0%, #08143a 55%, #05071b 100%)"}}
            >
                <div className="w-full max-w-[480px] mx-auto px-7 pt-10 pb-12 flex flex-col gap-7">
                    <div data-tauri-drag-region>
                        <h2 className="text-[20px] font-semibold tracking-tight text-white">
                            {activeTabInfo?.label}
                        </h2>
                    </div>

                    <div className="flex-1">
                        <AnimatePresence mode="wait">
                            <motion.div
                                key={activeTab}
                                initial={{opacity: 0, y: 8}}
                                animate={{opacity: 1, y: 0}}
                                exit={{opacity: 0, y: -8}}
                                transition={{duration: 0.18, ease: "easeOut"}}
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

function getAvatarColor(url: string | undefined): string | null {
    if (!url) return null
    try {
        const queryStart = url.indexOf("?")
        if (queryStart === -1) return null
        const params = new URLSearchParams(url.slice(queryStart))
        const r = params.get("r")
        const g = params.get("g")
        const b = params.get("b")
        if (r && g && b) return `rgb(${r}, ${g}, ${b})`
    } catch {
        return null
    }
    return null
}

function SidebarProfile() {
    const auth = core.useAuthentication()
    const user = auth?.user

    const initial = (user?.name?.trim()?.[0] ?? user?.email?.trim()?.[0] ?? "?").toUpperCase()
    const avatarColor = getAvatarColor(user?.avatar)
    const avatarStyle = avatarColor ? {backgroundColor: avatarColor} : undefined

    return (
        <div
            className="w-10 h-10 rounded-full border border-white/10 overflow-hidden flex items-center justify-center text-[14px] font-semibold text-white"
            style={avatarStyle}
        >
            {user?.avatar ? (
                <img
                    src={user.avatar}
                    alt=""
                    referrerPolicy="no-referrer"
                    className="w-full h-full object-cover"
                />
            ) : (
                <span>{initial}</span>
            )}
        </div>
    )
}

function SidebarItem({label, icon, active, onClick}: {
    label: string
    icon: React.ReactNode
    active: boolean
    onClick: () => void
}) {
    return (
        <button
            onClick={onClick}
            className={`
                flex flex-col items-center justify-center gap-1 py-2.5 px-1 rounded-lg w-full transition-colors duration-150
                ${active
                    ? "bg-white/[0.07] text-white"
                    : "text-white/55 hover:bg-white/[0.03] hover:text-white/90"
                }
            `}
        >
            {icon}
            <span className="text-[10.5px] font-medium tracking-tight">{label}</span>
        </button>
    )
}

function SettingsSection({title, children, description}: {
    title?: string
    children: React.ReactNode
    description?: string
}) {
    return (
        <div>
            {title && (
                <h3 className="text-[10px] font-semibold text-white/40 mb-3 uppercase tracking-[0.14em]">
                    {title}
                </h3>
            )}
            <div>
                {children}
            </div>
            {description && (
                <p className="mt-2 text-[11px] text-white/30 leading-relaxed">
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
            flex items-center justify-between gap-4 py-3.5
            ${!last ? "border-b border-white/[0.05]" : ""}
        `}>
            <div className="flex gap-3 items-start min-w-0">
                {icon && <div className="mt-0.5 text-white/55 shrink-0">{icon}</div>}
                <div className="flex flex-col min-w-0">
                    <span className="text-[13px] font-medium text-white/95">{label}</span>
                    {description && (
                        <span className="text-[11px] text-white/45 leading-snug mt-0.5">{description}</span>
                    )}
                </div>
            </div>
            <div className="flex items-center shrink-0">
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
        </div>
    )
}

type PlanKind = "free" | "paid"

type FreeLimits = {
    maxFilesPerTransfer: number
    transferLifetimeCapBytes: number
    maxVisibleShelves: number
    passwordEncryptionAllowed: boolean
}

function formatCount(n: number): string {
    return n === 0 ? "Unlimited" : n.toString()
}

function formatBytes(n: number): string {
    if (n === 0) return "No cap"
    const gib = n / (1024 * 1024 * 1024)
    if (gib >= 1) return Number.isInteger(gib) ? `${gib} GB` : `${gib.toFixed(1)} GB`
    const mib = n / (1024 * 1024)
    return Number.isInteger(mib) ? `${mib} MB` : `${mib.toFixed(1)} MB`
}

function PlanComparison({limits, onUpgrade}: {limits: FreeLimits; onUpgrade: () => Promise<unknown>}) {
    const payment = core.usePayment()
    const isPurchasing = payment?.is_loading ?? false
    const handlePurchaseClick = async () => {
        if (isPurchasing) return
        await onUpgrade()
    }
    const features: {label: string; freeNote: string}[] = [
        {label: "Unlimited files per transfer", freeNote: `${formatCount(limits.maxFilesPerTransfer)} on Free`},
        {label: "No transfer size cap", freeNote: `${formatBytes(limits.transferLifetimeCapBytes)} on Free`},
        {label: "Unlimited shelves", freeNote: `${formatCount(limits.maxVisibleShelves)} on Free`},
        {
            label: "Password-protected transfers",
            freeNote: limits.passwordEncryptionAllowed ? "Included" : "Not on Free",
        },
    ]

    return (
        <div
            className="relative overflow-hidden rounded-xl border border-white/[0.14]"
            style={{
                background: "rgba(255,255,255,0.10)",
                backdropFilter: "blur(24px) saturate(140%)",
                WebkitBackdropFilter: "blur(24px) saturate(140%)",
            }}
        >
            <div className="relative px-5 pt-6 pb-5">
                <div className="flex flex-col items-center mb-5">
                    <span className="text-[32px] font-bold text-white tabular-nums tracking-tight leading-none">$14.99</span>
                    <span className="text-[10.5px] text-white/70 mt-1.5 uppercase tracking-wider">Lifetime</span>
                </div>

                <div className="h-px bg-white/[0.12]" />

                <ul className="flex flex-col gap-2.5 py-4">
                    {features.map((f, i) => (
                        <li key={i} className="flex items-center gap-3">
                            <div className="w-[18px] h-[18px] rounded-full bg-blue-500/20 flex items-center justify-center shrink-0">
                                <Check className="w-[11px] h-[11px] text-blue-300" strokeWidth={3} />
                            </div>
                            <div className="flex items-center justify-between flex-1 min-w-0 gap-3">
                                <span className="text-[12.5px] text-white/90 leading-tight truncate">{f.label}</span>
                                <span className="text-[10.5px] text-white/35 leading-tight shrink-0 tabular-nums">{f.freeNote}</span>
                            </div>
                        </li>
                    ))}
                </ul>

                <Button
                    onClick={handlePurchaseClick}
                    disabled={isPurchasing}
                    className="w-full h-9 text-[13px] font-semibold bg-gradient-to-b from-blue-500 to-blue-600 hover:from-blue-400 hover:to-blue-500 text-white border-none rounded-lg shadow-[0_4px_16px_rgba(59,130,246,0.35),inset_0_1px_0_rgba(255,255,255,0.2)] transition-all disabled:opacity-70 disabled:cursor-not-allowed"
                >
                    {isPurchasing ? (
                        <span className="flex items-center justify-center gap-2">
                            <Loader2 className="w-4 h-4 animate-spin" />
                            Connecting to Apple…
                        </span>
                    ) : (
                        "Get lifetime access"
                    )}
                </Button>
            </div>
        </div>
    )
}

function PaidPlanNotice() {
    return (
        <div className="relative overflow-hidden rounded-xl border border-white/[0.06] bg-white/[0.02]">
            <div className="absolute inset-0 bg-gradient-to-b from-blue-500/[0.08] via-transparent to-transparent pointer-events-none" />
            <div className="relative px-4 py-4 flex items-center gap-3">
                <ProBadge size="sm" />
                <div className="flex flex-col flex-1 min-w-0">
                    <span className="text-[13.5px] font-semibold text-white tracking-tight">Bytover Pro</span>
                    <span className="text-[11px] text-white/45 mt-0.5">Lifetime · Thanks for supporting Bytover</span>
                </div>
                <Button
                    variant="secondary"
                    size="sm"
                    className="h-[28px] px-3 text-[11.5px] bg-white/[0.06] hover:bg-white/[0.1] text-white/85 border border-white/[0.08] rounded-full shrink-0"
                >
                    Manage plan
                </Button>
            </div>
        </div>
    )
}

function AccountContent({onSignOut}: {onSignOut: () => void}) {
    const auth = core.useAuthentication()
    const payment = core.usePayment()
    const caps = payment?.capabilities
    const user = auth?.user
    const currentPlan: PlanKind = (caps?.plan as unknown) === "Paid" ? "paid" : "free"
    const handleUpgrade = async () => {
        try {
            await invoke("purchase_premium")
        } catch (error) {
            console.error("[payment] purchase_premium dispatch failed:", error)
        }
    }

    const subscriptionBody = caps == null ? (
        <div className="px-4 py-5 text-[12px] text-white/50">Loading plan…</div>
    ) : currentPlan === "paid" ? (
        <PaidPlanNotice />
    ) : (
        <PlanComparison
            limits={{
                maxFilesPerTransfer: Number(caps.transfer_limits.max_files_per_transfer),
                transferLifetimeCapBytes: Number(caps.transfer_limits.total_transfer_bytes_lifetime_cap),
                maxVisibleShelves: Number(caps.presentation.max_visible_shelves),
                passwordEncryptionAllowed: caps.transfer_limits.password_encryption_allowed,
            }}
            onUpgrade={handleUpgrade}
        />
    )

    return (
        <div className="space-y-7">
            <SettingsSection title="Subscription Plan">
                {subscriptionBody}
            </SettingsSection>

            {user?.email && (
                <SettingsSection title="Preferred Email">
                    <div className="flex items-center py-3.5">
                        <span className="text-[13px] text-white/90 truncate flex-1">{user.email}</span>
                    </div>
                </SettingsSection>
            )}

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

function UpdatesContent({isChecking, status, onCheck, onInstall}: {
    isChecking: boolean
    status: UpdateStatus | null
    onCheck: () => void
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

                {status?.available && status?.store_url && (
                    <SettingsRow
                        label="New Version Available"
                        description={`Version ${status.version} is ready.`}
                        last={true}
                    >
                        <Button
                            size="sm"
                            onClick={onInstall}
                            className="h-[28px] px-4 text-[12px] bg-blue-600 hover:bg-blue-500 text-white border-none rounded-full"
                        >
                            Open in App Store
                        </Button>
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
        <div className="flex flex-col min-h-[340px]">
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
                    onClick={() => openUrl("https://github.com/midwess/bytover")}
                    className="w-full text-left"
                >
                    <SettingsRow
                        label="GitHub"
                        description="View the source on github.com/midwess/bytover."
                        icon={<Github className="w-4 h-4" />}
                    >
                        <ChevronRight className="w-4 h-4 text-white/20" />
                    </SettingsRow>
                </button>
                <button
                    onClick={() => openUrl("https://bytover.com/feedback")}
                    className="w-full text-left"
                >
                    <SettingsRow
                        label="Feedback"
                        description="Share your thoughts to help us improve."
                        icon={<MessageSquare className="w-4 h-4" />}
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

            <div className="mt-auto pt-12 flex flex-col items-center gap-1 opacity-30">
                <span className="text-sm">Powered by Midwess</span>
                <span className="text-xs">© 2026 Westrise</span>
            </div>
        </div>
    )
}

export default SettingsWindow
