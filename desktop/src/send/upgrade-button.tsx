import {Button} from "@/components/ui/button"
import {invoke} from "@tauri-apps/api/core"
import {noop} from "motion"
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
    TooltipProvider,
} from "@/components/animate-ui/primitives/animate/tooltip"

export function UpgradeButton({tooltipText = "You have exceeded the free limit"}: {tooltipText?: string}) {
    const handleClick = () => {
        invoke("show_settings_with_tab", {tab: "account"}).then(noop)
    }
    return (
        <TooltipProvider openDelay={200} closeDelay={100}>
            <Tooltip side="top" sideOffset={4}>
                <TooltipTrigger asChild>
                    <Button
                        onClick={handleClick}
                        className="bg-bluePrimary text-foreground shadow-lg hover:bg-bluePrimary/60 min-w-[100px] w-fit whitespace-nowrap px-3"
                    >
                        Upgrade to premium
                    </Button>
                </TooltipTrigger>
                <TooltipContent className="bg-popover text-popover-foreground text-xs px-2 py-1 rounded-md shadow-lg border whitespace-pre-line text-center">
                    {tooltipText}
                </TooltipContent>
            </Tooltip>
        </TooltipProvider>
    )
}
