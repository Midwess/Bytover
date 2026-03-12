import { useState } from "react"
import { Tabs, TabsList, TabsTrigger, TabsContents, TabsContent } from "@/components/animate-ui/components/tabs"
import { Card, CardContent } from "@/components/ui/card"
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
  const [password, setPassword] = useState("")
  const [isCopied, setIsCopied] = useState(false)
  const [isStarted, setIsStarted] = useState(false)

  const handleCopy = () => {
    setIsCopied(true)
    setTimeout(() => setIsCopied(false), 2000)
  }

  return (
    <div className="flex flex-col items-start w-full gap-2">
      <Card className="flex flex-row px-3 py-2 items-center justify-between w-full bg-[#1A1A1A] border-white/10 rounded-2xl shadow-sm">
        <div className="flex flex-col">
          <p className="text-[#A1A1AA] text-[10px] leading-tight">You're online as</p>
          <p className="text-white font-bold text-sm">Minh Tien Dang</p>
        </div>
        <div className="relative">
          <div className="w-9 h-9 rounded-xl border border-[#BEF264]/50 flex items-center justify-center bg-[#E5D5C5] overflow-hidden">
             <span className="text-xl">🦁</span>
          </div>
          <div className="absolute -bottom-0.5 -right-0.5 w-2.5 h-2.5 bg-[#BEF264] rounded-full border-2 border-[#1A1A1A]" />
        </div>
      </Card>

      <Card className="flex flex-row items-center px-3 py-2 w-full bg-[#1A1A1A] border-white/10 rounded-xl gap-2">
        <input
          className="bg-transparent border-none outline-none text-white text-sm flex-1 placeholder:text-[#3F3F46]"
          placeholder="Password (Optional)"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
        />
        <Eye className="w-4 h-4 text-[#3F3F46] cursor-pointer" />
      </Card>

      {isStarted && (
        <Card className="flex flex-row items-center px-3 py-2 w-full bg-[#1A1A1A] border-white/10 rounded-xl gap-2">
           <div className="flex-1 truncate">
             <span className="text-[9px] text-white/70">bytover.com/transfer?session=abc123</span>
           </div>
           <button onClick={handleCopy}>
             {isCopied ? <Check className="w-3 h-3 text-green-500" /> : <Copy className="w-3 h-3 text-white/50" />}
           </button>
        </Card>
      )}

      <Card className="p-1 bg-[#1A1A1A] border-white/10 rounded-xl w-fit">
        <Button 
          onClick={() => setIsStarted(!isStarted)}
          className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-4 py-1.5 h-auto text-sm font-semibold flex items-center gap-1.5"
        >
          {isStarted ? "Cancel" : "Start"} <ChevronRight className={cn("w-4 h-4", isStarted && "rotate-180")} />
        </Button>
      </Card>
    </div>
  )
}

function CloudTab() {
  return (
    <div className="flex flex-col gap-2 w-full">
      <Card className="bg-[#1A1A1A] border-white/10 p-3 rounded-xl">
        <p className="text-[10px] text-[#A1A1AA]">
          Create a sharable link. Files are stored for 7 days.
        </p>
      </Card>
      <Card className="flex flex-row items-center px-3 py-2 w-full bg-[#1A1A1A] border-white/10 rounded-xl gap-2">
        <input className="bg-transparent border-none outline-none text-white text-sm flex-1 placeholder:text-[#3F3F46]" placeholder="Password (Optional)" type="password" />
        <Eye className="w-4 h-4 text-[#3F3F46]" />
      </Card>
      <Card className="p-1 bg-[#1A1A1A] border-white/10 rounded-xl w-fit">
        <Button className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-4 py-1.5 h-auto text-sm font-semibold flex items-center gap-1.5">
          Upload <ChevronRight className="w-4 h-4" />
        </Button>
      </Card>
    </div>
  )
}

function EmailTab() {
  return (
    <div className="flex flex-col gap-2 w-full">
      <Card className="flex flex-row items-center px-3 py-2 w-full bg-[#1A1A1A] border-white/10 rounded-xl gap-2">
        <input className="bg-transparent border-none outline-none text-white text-sm flex-1 placeholder:text-[#3F3F46]" placeholder="Enter recipient emails" />
      </Card>
      <Card className="flex flex-row items-center px-3 py-2 w-full bg-[#1A1A1A] border-white/10 rounded-xl gap-2">
        <input className="bg-transparent border-none outline-none text-white text-sm flex-1 placeholder:text-[#3F3F46]" placeholder="Password (Optional)" type="password" />
        <Eye className="w-4 h-4 text-[#3F3F46]" />
      </Card>
      <Card className="p-1 bg-[#1A1A1A] border-white/10 rounded-xl w-fit">
        <Button className="bg-[#1D4ED8] hover:bg-[#1e40af] text-white rounded-lg px-4 py-1.5 h-auto text-sm font-semibold flex items-center gap-1.5">
          Send <Send className="w-3.5 h-3.5" />
        </Button>
      </Card>
    </div>
  )
}

export function SharingControlPanel({ className }: SharingControlPanelProps) {
  return (
    <div className={cn("flex w-full flex-col gap-4", className)}>
      <Tabs defaultValue="quick" className="w-full flex flex-col">
        <TabsList className="bg-[#111111] border border-white/10 p-1 rounded-xl w-full mb-3">
          <TabsTrigger 
            value="quick" 
            className="rounded-lg flex-1 px-1 py-1 data-[state=active]:bg-[#1A1A1A] data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
          >
            <Zap className="h-3 w-3" /> Quick
          </TabsTrigger>
          <TabsTrigger 
            value="cloud" 
            className="rounded-lg flex-1 px-1 py-1 data-[state=active]:bg-[#1A1A1A] data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
          >
            <Globe className="h-3 w-3" /> Cloud
          </TabsTrigger>
          <TabsTrigger 
            value="email" 
            className="rounded-lg flex-1 px-1 py-1 data-[state=active]:bg-[#1A1A1A] data-[state=active]:text-white text-[#A1A1AA] flex items-center justify-center gap-1 text-[10px] transition-all"
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


