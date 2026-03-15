import { useState } from "react"
import { useRouter } from "next/navigation"
import { Tabs, TabsList, TabsTrigger, TabsContents, TabsContent } from "@/components/animate-ui/components/tabs"
import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import {
  Zap,
  Globe,
  Mail,
  ChevronRight,
  Eye,
  Copy,
  Check,
  Send,
} from "lucide-react"
import { cn } from "@/lib/utils"

interface SharingControlPanelProps {
  className?: string
}

function P2PTab() {
  const router = useRouter()
  const [password, setPassword] = useState("")
  const [isCopied, setIsCopied] = useState(false)

  const handleCopy = () => {
    setIsCopied(true)
    setTimeout(() => setIsCopied(false), 2000)
  }

  const handleStart = () => {
    router.push("/transfer")
  }

  return (
    <div className="flex flex-col items-start w-full gap-1.5 text-left">
      <Card className="flex flex-row px-2 py-1.5 items-center justify-between w-full bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-2xl shadow-lg shadow-black/40">
        <div className="flex flex-col items-start text-left">
          <p className="text-[#A1A1AA] text-[9px] leading-tight">You&apos;re online as</p>
          <p className="text-white font-bold text-xs">John Doe</p>
        </div>
        <div className="relative">
          <div className="w-7 h-7 rounded-xl border border-[#BEF264]/50 flex items-center justify-center bg-[#E5D5C5] overflow-hidden">
             <span className="text-lg">🦁</span>
          </div>
          <div className="absolute -bottom-0.5 -right-0.5 w-2 h-2 bg-[#BEF264] rounded-full border-2 border-zinc-900" />
        </div>
      </Card>

      <Card className="flex flex-row items-center px-2 py-1.5 w-full bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl gap-2 shadow-lg shadow-black/40">
        <input
          className="bg-transparent border-none outline-none text-white text-xs flex-1 placeholder:text-[#3F3F46]"
          placeholder="Password (Optional)"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        <Eye className="w-3.5 h-3.5 text-[#3F3F46] cursor-pointer" />
      </Card>

      <Card className="p-1 bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl w-fit shadow-lg shadow-black/40">
        <Button
          onClick={handleStart}
          className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-2 py-1 h-auto text-[11px] font-semibold flex items-center gap-1"
        >
          Start <ChevronRight className="w-3 h-3" />
        </Button>
      </Card>
    </div>
  )
}

function CloudTab() {
  return (
    <div className="flex flex-col gap-1.5 w-full">
      <Card className="bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 p-2 rounded-xl shadow-lg shadow-black/40">
        <p className="text-[9px] text-[#A1A1AA] text-left">
          Create a sharable link. Files are stored for 7 days.
        </p>
      </Card>
      <Card className="flex flex-row items-center px-2 py-1.5 w-full bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl gap-2 shadow-lg shadow-black/40">
        <input className="bg-transparent border-none outline-none text-white text-xs flex-1 placeholder:text-[#3F3F46]" placeholder="Password (Optional)" type="password" />
        <Eye className="w-3.5 h-3.5 text-[#3F3F46]" />
      </Card>
      <Card className="p-1 bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl w-fit shadow-lg shadow-black/40">
        <Button className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-2 py-1 h-auto text-[11px] font-semibold flex items-center gap-1">
          Upload <ChevronRight className="w-3 h-3" />
        </Button>
      </Card>
    </div>
  )
}

function EmailTab() {
  return (
    <div className="flex flex-col gap-1.5 w-full">
      <Card className="flex flex-row items-center px-2 py-1.5 w-full bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl gap-2 shadow-lg shadow-black/40">
        <input className="bg-transparent border-none outline-none text-white text-xs flex-1 placeholder:text-[#3F3F46]" placeholder="Enter recipient emails" />
      </Card>
      <Card className="flex flex-row items-center px-2 py-1.5 w-full bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl gap-2 shadow-lg shadow-black/40">
        <input className="bg-transparent border-none outline-none text-white text-xs flex-1 placeholder:text-[#3F3F46]" placeholder="Password (Optional)" type="password" />
        <Eye className="w-3.5 h-3.5 text-[#3F3F46]" />
      </Card>
      <Card className="p-1 bg-[#1A1A1A]/80 backdrop-blur-md border-white/10 rounded-xl w-fit shadow-lg shadow-black/40">
        <Button className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-2 py-1 h-auto text-[11px] font-semibold flex items-center gap-1">
          Send <Send className="w-3 h-3" />
        </Button>
      </Card>
    </div>
  )
}

export function SharingControlPanel({ className }: SharingControlPanelProps) {
  return (
    <div className={cn("flex w-full flex-col gap-1.5", className)}>
      <Tabs defaultValue="quick" className="w-full flex flex-col">
        <TabsList className="bg-transparent backdrop-blur-md border border-white/10 p-0.5 rounded-xl w-full mb-1">
          <TabsTrigger 
            value="quick" 
            className="rounded-lg flex-1 px-0.5 py-1 data-[state=active]:bg-white/10 data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
          >
            <Zap className="h-3 w-3" /> Quick
          </TabsTrigger>
          <TabsTrigger 
            value="cloud" 
            className="rounded-lg flex-1 px-0.5 py-1 data-[state=active]:bg-white/10 data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
          >
            <Globe className="h-3 w-3" /> Cloud
          </TabsTrigger>
          <TabsTrigger 
            value="email" 
            className="rounded-lg flex-1 px-0.5 py-1 data-[state=active]:bg-white/10 data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
          >
            <Mail className="h-3 w-3" /> Email
          </TabsTrigger>
        </TabsList>
        <div className="w-full">
          <TabsContents>
            <TabsContent value="quick">
              <P2PTab />
            </TabsContent>
            <TabsContent value="cloud">
              <CloudTab />
            </TabsContent>
            <TabsContent value="email">
              <EmailTab />
            </TabsContent>
          </TabsContents>
        </div>
      </Tabs>
    </div>
  )
}


