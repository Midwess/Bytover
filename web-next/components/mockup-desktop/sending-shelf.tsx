import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import {
  MoreVertical,
  Plus,
  Minus,
  FolderIcon,
  FileIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"
import React from "react"

export type MockFile = {
  id: string
  name: string
  size: string
  type: "file" | "folder" | "image" | "video"
  thumbnailUrl?: string
}

const mockFiles: MockFile[] = [
  { id: "1", name: "documents", size: "19.12 MB", type: "folder", thumbnailUrl: "/demo/image1.jpg" },
  { id: "2", name: "certific...on.pdf", size: "0.52 MB", type: "file", thumbnailUrl: "/demo/image2.jpg" },
  { id: "3", name: "certific...on.pdf", size: "0.53 MB", type: "file", thumbnailUrl: "/demo/image3.jpg" },
  { id: "4", name: "Airwallex.pdf", size: "0.12 MB", type: "file" },
]

function ResourceView({ file }: { file: MockFile }) {
  const isFolder = file.type === "folder"
  return (
    <Card className="w-full border bg-[#1A1A1A]/80 backdrop-blur-md rounded-xl flex flex-row items-center p-1 pl-1.5 pr-0 relative group transition-colors cursor-pointer hover:bg-[#1A1A1A] border-white/10 justify-between">
      <div className="flex flex-row items-center gap-1.5 flex-1 min-w-0">
        <div className="w-9 h-9 shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden flex items-center justify-center transition-all">
          {file.thumbnailUrl ? (
            <img
              src={file.thumbnailUrl}
              alt={file.name}
              className="w-full h-full object-cover rounded-md"
            />
          ) : isFolder ? (
            <FolderIcon className="w-5 h-5 text-primary fill-primary/20" />
          ) : (
            <FileIcon className="w-5 h-5 text-primary" />
          )}
        </div>
        <div className="flex-1 min-w-0 flex flex-col justify-center items-start text-left">
          <p className="text-[11px] font-medium text-white truncate leading-tight w-full text-left">{file.name}</p>
          <p className="text-[9px] text-white/60 leading-tight w-full text-left">{file.size}</p>
        </div>
      </div>
      <Button variant="ghost" className="p-0 h-8 w-6 hover:bg-transparent text-white/40 shrink-0 flex items-center justify-center">
        <MoreVertical className="w-3.5 h-3.5" />
      </Button>
    </Card>
  )
}

interface SendingShelfProps {
  className?: string
  isDraggingOver?: boolean
}

export function SendingShelf({ className, isDraggingOver = false }: SendingShelfProps) {
  return (
    <Card
      className={cn(
        "rounded-[30px] flex flex-col bg-[#111111]/80 backdrop-blur-xl border border-white/10 p-0 w-full h-full relative overflow-hidden",
        isDraggingOver && "border-blue-500 shadow-[0_0_8px_2px_rgba(59,130,246,0.5)_inset]",
        className
      )}
    >
      <div className="absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-black/20 to-transparent pointer-events-none z-20" />
      
      {/* Drag handle matching desktop/src/send/shelf.tsx */}
      <div
        className="w-full py-1 absolute top-0 flex justify-center items-center z-[60] peer group flex-col cursor-pointer"
      >
        <Minus className="pointer-events-none scale-x-200 scale-y-200 text-white/50 transition-transform duration-200 group-hover:scale-x-[3] group-hover:scale-y-[2.5]" />
      </div>

      {isDraggingOver && (
        <div className="absolute inset-0 bg-blue-500/10 backdrop-blur-[3px] flex items-center justify-center z-40">
          <div className="flex flex-col items-center gap-2 text-primary">
            <Plus className="h-9 w-10 text-blue-500" />
          </div>
        </div>
      )}

      <div 
        className="w-full h-full overflow-y-auto px-1.5 z-0 pt-6 no-scrollbar"
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        style={{ overflowY: 'overlay' } as any}
      >
        <div className="flex flex-col gap-1">
          {mockFiles.map((file) => (
            <ResourceView key={file.id} file={file} />
          ))}
          <div className="h-4" />
        </div>
      </div>

      <style jsx>{`
        .no-scrollbar::-webkit-scrollbar {
          display: none;
        }
        .no-scrollbar {
          -ms-overflow-style: none;
          scrollbar-width: none;
        }
      `}</style>
    </Card>
  )
}


