interface ConvertHeifMessage {
  type: 'CONVERT_HEIF'
  id: string
  heifData: ArrayBuffer
  quality: number
}

interface WorkerResponse {
  type: 'HEIF_SUCCESS' | 'HEIF_ERROR'
  id: string
  data?: ArrayBuffer
  error?: string
}

class ImageWorker {
  private worker: Worker | null = null
  private pendingTasks = new Map<string, {
    resolve: (file: File) => void
    reject: (error: Error) => void
  }>()

  constructor() {
    if (typeof window !== 'undefined') {
      this.worker = new Worker(new URL('@/app/workers/image.worker.ts', import.meta.url))
      this.worker.onmessage = (event: MessageEvent<WorkerResponse>) => {
        const { type, id, data, error } = event.data
        const task = this.pendingTasks.get(id)
        
        if (!task) return
        this.pendingTasks.delete(id)

        if (type === 'HEIF_SUCCESS' && data) {
          const file = new File([data], `converted.png`, { type: 'image/png' })
          task.resolve(file)
        } else {
          task.reject(new Error(error || 'Unknown error'))
        }
      }
    }
  }

  async convertHeif(file: File, quality: number = 0.2): Promise<File> {
    if (!this.worker) throw new Error('Worker not available')

    const id = `heif_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`
    const heifData = await file.arrayBuffer()

    return new Promise<File>((resolve, reject) => {
      this.pendingTasks.set(id, { resolve, reject })

      const message: ConvertHeifMessage = {
        type: 'CONVERT_HEIF',
        id,
        heifData,
        quality
      }

      console.log('posting message', message)
      this.worker!.postMessage(message)
    })
  }
}

const imageWorker = new ImageWorker()
export { imageWorker }
