import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import {
  MoreVertical,
  Plus,
  Trash2,
  ClipboardPaste,
  Minus,
  MoreHorizontal,
  FolderIcon,
  FileIcon,
} from "lucide-react"
import { cn } from "@/lib/utils"

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
    <Card className="w-full border bg-muted/50 rounded-xl flex flex-row items-center p-1 relative group transition-colors cursor-pointer gap-2 hover:bg-muted-foreground/30 border-white/10">
      <div className="w-12 h-12 shrink-0 rounded-lg bg-muted-foreground/15 p-1 overflow-hidden flex items-center justify-center transition-all">
        {file.thumbnailUrl ? (
          <img
            src={file.thumbnailUrl}
            alt={file.name}
            className="w-full h-full object-cover rounded-md"
          />
        ) : isFolder ? (
          <FolderIcon className="w-6 h-6 text-primary fill-primary/20" />
        ) : (
          <FileIcon className="w-6 h-6 text-primary" />
        )}
      </div>
      <div className="flex-1 min-w-0 flex flex-col justify-center">
        <p className="text-sm font-medium text-white truncate">{file.name}</p>
        <p className="text-xs text-white/70">{file.size}</p>
      </div>
      <Button variant="ghost" className="p-0 h-auto w-auto hover:bg-transparent text-white/70">
        <MoreVertical className="w-4 h-4" />
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
        "rounded-[30px] flex flex-col bg-[#111111] border border-white/20 p-0 w-full h-full relative overflow-hidden",
        isDraggingOver && "border-blue-500 shadow-[0_0_8px_2px_rgba(59,130,246,0.5)_inset]",
        className
      )}
    >
      <div className="absolute top-0 left-0 right-0 h-5 bg-gradient-to-b from-[#111111] to-transparent pointer-events-none z-20" />
      
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

      {/* Resources List - matching desktop layout */}
      <div className="w-full h-full overflow-y-auto px-2 z-0 pt-7 no-scrollbar">
        <div className="flex flex-col gap-1.5">
          {mockFiles.map((file) => (
            <ResourceView key={file.id} file={file} />
          ))}
          <div className="h-4" />
        </div>
      </div>

      {/* Bottom controls - matching desktop layout */}
      <div className="absolute bottom-0 left-0 right-0 h-fit bg-gradient-to-t from-[#111111] to-transparent z-20 w-full justify-center flex flex-row pb-2">
        <div className="group z-20 flex flex-col items-center justify-end bg-transparent text-muted-foreground transition-all duration-500 ease-out hover:pb-2 gap-2">
          <div className="flex flex-col gap-1.5 overflow-hidden max-h-0 opacity-0 transition-all duration-300 ease-out group-hover:max-h-24 group-hover:opacity-100 group-hover:mb-1">
            <Button
              variant="ghost"
              size="sm"
              className="w-24 flex items-center justify-center gap-1.5 text-foreground text-xs bg-muted/90 px-2 py-1 h-auto rounded-lg border border-white/10"
            >
              <Trash2 className="h-3.5 w-3.5" />
              <span>Clear all</span>
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className="flex items-center justify-center gap-1.5 text-foreground text-xs bg-muted/90 px-2 py-1 h-auto w-24 rounded-lg border border-white/10"
            >
              <ClipboardPaste className="h-3.5 w-3.5" />
              <span>Paste</span>
            </Button>
          </div>
          <MoreHorizontal className="h-7 w-7 flex-shrink-0 transition-transform text-white p-[2px] duration-500 ease-out bg-muted/90 rounded-full cursor-pointer" />
        </div>
      </div>
    </Card>
  )
}


