import ReactDOM from "react-dom/client"
import React, {useEffect, useState, useRef, useCallback} from "react"
import {X} from "lucide-react"
import {listen} from "@tauri-apps/api/event"
import {invoke} from "@tauri-apps/api/core"

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
    <React.StrictMode>
        <ToastWindow/>
    </React.StrictMode>,
)

function ToastWindow() {
    const [message, setMessage] = useState<string>("")
    const [visible, setVisible] = useState(false)
    const timerRef = useRef<NodeJS.Timeout | null>(null)

    const startTimer = useCallback(() => {
        if (timerRef.current) {
            clearTimeout(timerRef.current)
        }
        timerRef.current = setTimeout(() => {
            handleClose()
        }, 8000)
    }, [])

    const showToast = useCallback(async () => {
        const msg = await invoke<string | null>("get_toast_message")
        if (msg) {
            setMessage(msg)
            setVisible(true)
            startTimer()
        }
    }, [startTimer])

    const handleClose = async () => {
        setVisible(false)
        if (timerRef.current) {
            clearTimeout(timerRef.current)
            timerRef.current = null
        }
        try {
            await invoke("close_toast")
        } catch (e) {
            console.error("Failed to close toast", e)
        }
    }

    useEffect(() => {
        showToast()

        const unlisten = listen<string>("toast-message", () => {
            showToast()
        })

        return () => {
            unlisten.then(fn => fn())
            if (timerRef.current) {
                clearTimeout(timerRef.current)
            }
        }
    }, [showToast])

    if (!visible || !message) {
        return null
    }

    return (
        <main className="w-screen h-screen dark">
            <div className="w-full h-full bg-black/10 rounded-[22px] flex flex-row items-center justify-between px-4 border border-white/10">
                <p className="text-white text-sm flex-1 truncate pr-3">{message}</p>
                <button
                    onClick={handleClose}
                    className="text-zinc-400 hover:text-white transition-colors p-1 rounded-lg hover:bg-white/10"
                >
                    <X size={18}/>
                </button>
            </div>
        </main>
    )
}

export default ToastWindow
