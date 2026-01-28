import {PeerAvatarViewModel} from "shared_types/types/shared_types";
import {Avatar, AvatarFallback, AvatarImage} from "@/components/ui/avatar";
import {
    Tooltip,
    TooltipContent,
    TooltipTrigger,
    TooltipProvider,
} from "@/components/animate-ui/primitives/animate/tooltip";
import {Check} from "lucide-react";
import {useMemo} from "react";

interface PeerAvatarGroupProps {
    peers: PeerAvatarViewModel[];
    maxDisplay?: number;
}

function extractDominantColor(url: string): { r: number; g: number; b: number } | null {
    try {
        const queryStart = url.indexOf('?');
        if (queryStart === -1) return null;

        const params = new URLSearchParams(url.slice(queryStart));
        const r = params.get('r');
        const g = params.get('g');
        const b = params.get('b');

        if (r && g && b) {
            return {
                r: parseInt(r, 10),
                g: parseInt(g, 10),
                b: parseInt(b, 10)
            };
        }
    } catch {
        return null;
    }
    return null;
}

export function PeerAvatarGroup({peers, maxDisplay = 4}: PeerAvatarGroupProps) {
    if (!peers || peers.length === 0) return null;

    const displayedPeers = peers.slice(0, maxDisplay);
    const overflowCount = peers.length - maxDisplay;

    const backgroundColor = useMemo(() => {
        const firstPeer = peers[0];
        if (!firstPeer?.avatar_url) return undefined;

        const color = extractDominantColor(firstPeer.avatar_url);
        if (color) {
            return `rgba(${color.r}, ${color.g}, ${color.b}, 0.25)`;
        }
        return undefined;
    }, [peers]);

    return (
        <TooltipProvider openDelay={300} closeDelay={100}>
            <div className="pt-0.5">
                <div
                    className={`inline-flex items-center gap-0.5 border border-border/50 rounded-full py-px px-0.5 ${!backgroundColor ? 'bg-background/60' : ''}`}
                    style={backgroundColor ? {backgroundColor} : undefined}
                >
                    {/* Avatar stack */}
                    <div className="flex items-center -space-x-1">
                        {displayedPeers.map((peer, index) => (
                            <Tooltip key={index} side="top" sideOffset={4}>
                                <TooltipTrigger asChild>
                                    <div style={{zIndex: displayedPeers.length - index}}>
                                        <Avatar className="h-4 w-4 cursor-pointer hover:opacity-80 transition-opacity">
                                            <AvatarImage src={peer.avatar_url} alt={peer.name}/>
                                            <AvatarFallback className="text-[6px] bg-muted-foreground/50 text-primary font-medium">
                                                {peer.name.charAt(0).toUpperCase()}
                                            </AvatarFallback>
                                        </Avatar>
                                    </div>
                                </TooltipTrigger>
                                <TooltipContent className="bg-popover text-popover-foreground text-xs px-2 py-1 rounded-md shadow-lg border">
                                    {peer.name}
                                </TooltipContent>
                            </Tooltip>
                        ))}
                        {overflowCount > 0 && (
                            <Tooltip side="top" sideOffset={4}>
                                <TooltipTrigger asChild>
                                    <div
                                        className="h-4 w-4 rounded-full bg-muted-foreground/40 flex items-center justify-center cursor-pointer hover:bg-muted-foreground/60 transition-colors"
                                        style={{zIndex: 0}}
                                    >
                                        <span className="text-[6px] text-primary font-medium">+{overflowCount}</span>
                                    </div>
                                </TooltipTrigger>
                                <TooltipContent className="bg-popover text-popover-foreground text-xs px-2 py-1 rounded-md shadow-lg border">
                                    {peers.slice(maxDisplay).map(p => p.name).join(", ")}
                                </TooltipContent>
                            </Tooltip>
                        )}
                    </div>

                    {/* Checkmark */}
                    <Check className="h-2.5 w-2.5 text-green-500 shrink-0 mr-0.5"/>
                </div>
            </div>
        </TooltipProvider>
    );
}
