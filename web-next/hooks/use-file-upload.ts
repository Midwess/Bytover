"use client"

import type React from "react"
import {
  useCallback,
  useRef,
  useState,
  useEffect,
  type ChangeEvent,
  type DragEvent,
  type InputHTMLAttributes,
} from "react"

export type FileMetadata = {
  name: string
  size: number
  type: string
  url: string
  id: string
}

export type FileWithPreview = {
  file: File | FileMetadata
  id: string
  preview?: string
}

export type FolderStructure = {
  folderName: string // Root folder name from webkitRelativePath
  files: FileWithPreview[] // Files with their original webkitRelativePath preserved
  id: string // Unique identifier for this folder
}

export type FileUploadOptions = {
  maxFiles?: number
  maxSize?: number // in bytes
  accept?: string
  multiple?: boolean
  allowDirectories?: boolean
  initialFiles?: FileMetadata[]
  onFilesChange?: (files: FileWithPreview[]) => void
  onFoldersChange?: (folders: FolderStructure[]) => void
  onFilesAdded?: (addedFiles: FileWithPreview[]) => void
  onFoldersAdded?: (addedFolders: FolderStructure[]) => void
}

export type FileUploadState = {
  files: FileWithPreview[] // Individual files
  folders: FolderStructure[] // Folders with their files
  isDragging: boolean
  errors: string[]
  supportsDirectories: boolean
}

export type FileUploadActions = {
  addFiles: (files: FileList | File[]) => void
  addFolders: (files: FileList | File[] | Promise<File[]>) => Promise<void>
  removeFile: (id: string) => void
  removeFolder: (id: string) => void
  clearFiles: () => void
  clearFolders: () => void
  clearAll: () => void
  clearErrors: () => void
  handleDragEnter: (e: DragEvent<HTMLElement>) => void
  handleDragLeave: (e: DragEvent<HTMLElement>) => void
  handleDragOver: (e: DragEvent<HTMLElement>) => void
  handleDrop: (e: DragEvent<HTMLElement>) => void
  handleFileChange: (e: ChangeEvent<HTMLInputElement>) => void
  handleFolderChange: (e: ChangeEvent<HTMLInputElement>) => void
  openFileDialog: () => void
  openDirectoryDialog: () => void
  getInputProps: (
      props?: InputHTMLAttributes<HTMLInputElement>
  ) => InputHTMLAttributes<HTMLInputElement> & {
    ref: React.Ref<HTMLInputElement>
  }
  getDirectoryInputProps: (
      props?: InputHTMLAttributes<HTMLInputElement>
  ) => InputHTMLAttributes<HTMLInputElement> & {
    ref: React.Ref<HTMLInputElement>
  }
}

export const useFileUpload = (
    options: FileUploadOptions = {}
): [FileUploadState, FileUploadActions] => {
  const {
    maxFiles = Infinity,
    maxSize = Infinity,
    accept = "*",
    multiple = false,
    allowDirectories = false,
    initialFiles = [],
    onFilesChange,
    onFoldersChange,
    onFilesAdded,
    onFoldersAdded,
  } = options

  const supportsDirectories = useCallback(() => {
    return 'webkitdirectory' in document.createElement('input')
  }, [])

  const [state, setState] = useState<FileUploadState>({
    files: initialFiles.map((file) => ({
      file,
      id: file.id,
      preview: file.url,
    })),
    folders: [],
    isDragging: false,
    errors: [],
    supportsDirectories: false,
  })

  const inputRef = useRef<HTMLInputElement>(null)
  const directoryInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    setState(prev => ({
      ...prev,
      supportsDirectories: supportsDirectories()
    }))
  }, [supportsDirectories])

  const validateFile = useCallback(
      (file: File | FileMetadata): string | null => {
        if (file instanceof File) {
          if (file.size > maxSize) {
            return `File "${file.name}" exceeds the maximum size of ${formatBytes(maxSize)}.`
          }
        } else {
          if (file.size > maxSize) {
            return `File "${file.name}" exceeds the maximum size of ${formatBytes(maxSize)}.`
          }
        }

        if (accept !== "*") {
          const acceptedTypes = accept.split(",").map((type) => type.trim())
          const fileType = file instanceof File ? file.type || "" : file.type
          const fileExtension = `.${file instanceof File ? file.name.split(".").pop() : file.name.split(".").pop()}`

          const isAccepted = acceptedTypes.some((type) => {
            if (type.startsWith(".")) {
              return fileExtension.toLowerCase() === type.toLowerCase()
            }
            return fileType === type
          })

          if (!isAccepted) {
            return `File "${file instanceof File ? file.name : file.name}" is not an accepted file type.`
          }
        }

        return null
      },
      [accept, maxSize]
  )

  const createPreview = useCallback(
      (file: File | FileMetadata): string | undefined => {
        if (file instanceof File) {
          return URL.createObjectURL(file)
        }
        return file.url
      },
      []
  )

  const generateUniqueId = useCallback((file: File | FileMetadata): string => {
    if (file instanceof File) {
      return `${file.name}-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
    }
    return file.id
  }, [])

  const generateFolderId = useCallback((folderName: string): string => {
    return `folder-${folderName}-${Date.now()}-${Math.random().toString(36).substring(2, 9)}`
  }, [])

  // Enhanced folder organization that handles both webkitRelativePath and custom paths
  const organizeFolderStructure = useCallback((files: File[]): FolderStructure[] => {
    const folderMap = new Map<string, File[]>()

    files.forEach(file => {
      const webkitRelativePath = (file as any).webkitRelativePath || ""
      if (webkitRelativePath) {
        // Get root folder name (first part of path)
        const folderName = webkitRelativePath.split("/")[0]

        if (!folderMap.has(folderName)) {
          folderMap.set(folderName, [])
        }
        folderMap.get(folderName)!.push(file)
      }
    })

    return Array.from(folderMap.entries()).map(([folderName, files]) => ({
      folderName,
      files: files.map(file => ({
        file,
        id: generateUniqueId(file),
        preview: createPreview(file),
      })),
      id: generateFolderId(folderName),
    }))
  }, [generateUniqueId, generateFolderId, createPreview])

  // Helper function to traverse directory entries and collect files with proper paths
  const traverseDirectoryEntry = useCallback(async (entry: any, basePath = ""): Promise<File[]> => {
    return new Promise((resolve, reject) => {
      const collectedFiles: File[] = []
      let pendingOperations = 0
      let completed = false

      const addOperation = () => {
        pendingOperations++
      }

      const completeOperation = () => {
        pendingOperations--
        if (pendingOperations === 0 && !completed) {
          completed = true
          resolve(collectedFiles)
        }
      }

      const traverseEntry = (dirEntry: any, currentPath: string) => {
        addOperation()
        const reader = dirEntry.createReader()

        const readEntries = () => {
          reader.readEntries((entries: any[]) => {
            if (entries.length === 0) {
              completeOperation()
              return
            }

            entries.forEach((entryItem) => {
              if (entryItem.isFile) {
                addOperation()
                entryItem.file((file: File) => {
                  // Set the webkitRelativePath to match the structure expected by organizeFolderStructure
                  const relativePath = currentPath ? `${currentPath}/${file.name}` : file.name;
                  Object.defineProperty(file, "webkitRelativePath", {
                    value: relativePath,
                    configurable: true,
                    enumerable: true,
                    writable: false, // keep read-only semantics
                  });
                  collectedFiles.push(file)
                  completeOperation()
                }, (error: any) => {
                  console.error('Error reading file:', error)
                  completeOperation()
                })
              } else if (entryItem.isDirectory) {
                const newPath = currentPath ? `${currentPath}/${entryItem.name}` : entryItem.name
                traverseEntry(entryItem, newPath)
              }
            })

            // Continue reading if there might be more entries
            readEntries()
          }, (error: any) => {
            console.error('Error reading directory entries:', error)
            completeOperation()
          })
        }

        readEntries()
      }

      if (entry.isDirectory) {
        const rootPath = basePath || entry.name
        traverseEntry(entry, rootPath)
      } else if (entry.isFile) {
        // Single file
        addOperation()
        entry.file((file: File) => {
          const relativePath = basePath ? `${basePath}/${file.name}` : file.name;
          Object.defineProperty(file, "webkitRelativePath", {
            value: relativePath,
            configurable: true,
            enumerable: true,
            writable: false,
          });
          collectedFiles.push(file)
          completeOperation()
        }, (error: any) => {
          console.error('Error reading file:', error)
          completeOperation()
        })
      } else {
        // Unknown entry type
        resolve([])
      }

      // Timeout fallback to prevent hanging
      setTimeout(() => {
        if (!completed) {
          completed = true
          console.warn('Directory traversal timed out, returning collected files')
          resolve(collectedFiles)
        }
      }, 10000)
    })
  }, [])

  const clearFiles = useCallback(() => {
    setState((prev) => {
      prev.files.forEach((file) => {
        if (
            file.preview &&
            file.file instanceof File &&
            file.file.type.startsWith("image/")
        ) {
          URL.revokeObjectURL(file.preview)
        }
      })

      if (inputRef.current) {
        inputRef.current.value = ""
      }

      const newState = {
        ...prev,
        files: [],
        errors: [],
      }

      onFilesChange?.(newState.files)
      return newState
    })
  }, [onFilesChange])

  const clearFolders = useCallback(() => {
    setState((prev) => {
      prev.folders.forEach((folder) => {
        folder.files.forEach((file) => {
          if (
              file.preview &&
              file.file instanceof File &&
              file.file.type.startsWith("image/")
          ) {
            URL.revokeObjectURL(file.preview)
          }
        })
      })

      if (directoryInputRef.current) {
        directoryInputRef.current.value = ""
      }

      const newState = {
        ...prev,
        folders: [],
        errors: [],
      }

      onFoldersChange?.(newState.folders)
      return newState
    })
  }, [onFoldersChange])

  const clearAll = useCallback(() => {
    clearFiles()
    clearFolders()
  }, [clearFiles, clearFolders])

  const addFiles = useCallback(
      (newFiles: FileList | File[]) => {
        if (!newFiles || newFiles.length === 0) return

        const newFilesArray = Array.from(newFiles)
        const errors: string[] = []

        setState((prev) => ({ ...prev, errors: [] }))

        if (!multiple) {
          clearFiles()
        }

        if (
            multiple &&
            maxFiles !== Infinity &&
            state.files.length + newFilesArray.length > maxFiles
        ) {
          errors.push(`You can only upload a maximum of ${maxFiles} files.`)
          setState((prev) => ({ ...prev, errors }))
          return
        }

        const validFiles: FileWithPreview[] = []

        newFilesArray.forEach((file) => {
          if (multiple) {
            const isDuplicate = state.files.some(
                (existingFile) =>
                    existingFile.file.name === file.name &&
                    existingFile.file.size === file.size
            )

            if (isDuplicate) {
              return
            }
          }

          if (file.size > maxSize) {
            errors.push(
                multiple
                    ? `Some files exceed the maximum size of ${formatBytes(maxSize)}.`
                    : `File exceeds the maximum size of ${formatBytes(maxSize)}.`
            )
            return
          }

          const error = validateFile(file)
          if (error) {
            errors.push(error)
          } else {
            validFiles.push({
              file,
              id: generateUniqueId(file),
              preview: createPreview(file),
            })
          }
        })

        if (validFiles.length > 0) {
          onFilesAdded?.(validFiles)

          setState((prev) => {
            const newFiles = !multiple ? validFiles : [...prev.files, ...validFiles]
            onFilesChange?.(newFiles)
            return {
              ...prev,
              files: newFiles,
              errors,
            }
          })
        } else if (errors.length > 0) {
          setState((prev) => ({
            ...prev,
            errors,
          }))
        }

        if (inputRef.current) {
          inputRef.current.value = ""
        }
      },
      [
        state.files,
        maxFiles,
        multiple,
        maxSize,
        validateFile,
        createPreview,
        generateUniqueId,
        clearFiles,
        onFilesChange,
        onFilesAdded,
      ]
  )

  // Enhanced addFolders function that handles both file input and drag/drop
  const addFolders = useCallback(
      async (newFiles: FileList | File[] | Promise<File[]>) => {
        try {
          // Handle async file collection from drag/drop
          const resolvedFiles = await Promise.resolve(newFiles)

          if (!resolvedFiles || resolvedFiles.length === 0) return

          const newFilesArray = Array.from(resolvedFiles)
          const errors: string[] = []

          setState((prev) => ({ ...prev, errors: [] }))

          // Validate files
          const validFiles: File[] = []
          newFilesArray.forEach((file) => {
            if (file.size > maxSize) {
              errors.push(`Some files in folders exceed the maximum size of ${formatBytes(maxSize)}.`)
              return
            }

            const error = validateFile(file)
            if (error) {
              errors.push(error)
            } else {
              validFiles.push(file)
            }
          })

          if (validFiles.length > 0) {
            const newFolders = organizeFolderStructure(validFiles)

            // Filter out duplicate folders
            const filteredFolders = newFolders.filter(newFolder => {
              return !state.folders.some(f => f.folderName === newFolder.folderName)
            })

            if (filteredFolders.length > 0) {
              onFoldersAdded?.(filteredFolders)

              setState((prev) => {
                const newFoldersState = [...prev.folders, ...filteredFolders]
                onFoldersChange?.(newFoldersState)
                return {
                  ...prev,
                  folders: newFoldersState,
                  errors,
                }
              })
            }
          } else if (errors.length > 0) {
            setState((prev) => ({
              ...prev,
              errors,
            }))
          }

          if (directoryInputRef.current) {
            directoryInputRef.current.value = ""
          }
        } catch (error) {
          console.error('Error processing folders:', error)
          setState((prev) => ({
            ...prev,
            errors: [...prev.errors, 'Error processing folders.']
          }))
        }
      },
      [
        state.folders,
        maxSize,
        validateFile,
        organizeFolderStructure,
        onFoldersChange,
        onFoldersAdded,
      ]
  )

  const removeFile = useCallback(
      (id: string) => {
        setState((prev) => {
          const fileToRemove = prev.files.find((file) => file.id === id)
          if (
              fileToRemove &&
              fileToRemove.preview &&
              fileToRemove.file instanceof File &&
              fileToRemove.file.type.startsWith("image/")
          ) {
            URL.revokeObjectURL(fileToRemove.preview)
          }

          const newFiles = prev.files.filter((file) => file.id !== id)
          onFilesChange?.(newFiles)

          return {
            ...prev,
            files: newFiles,
            errors: [],
          }
        })
      },
      [onFilesChange]
  )

  const removeFolder = useCallback(
      (id: string) => {
        setState((prev) => {
          const folderToRemove = prev.folders.find((folder) => folder.id === id)
          if (folderToRemove) {
            folderToRemove.files.forEach((file) => {
              if (
                  file.preview &&
                  file.file instanceof File &&
                  file.file.type.startsWith("image/")
              ) {
                URL.revokeObjectURL(file.preview)
              }
            })
          }

          const newFolders = prev.folders.filter((folder) => folder.id !== id)
          onFoldersChange?.(newFolders)

          return {
            ...prev,
            folders: newFolders,
            errors: [],
          }
        })
      },
      [onFoldersChange]
  )

  const clearErrors = useCallback(() => {
    setState((prev) => ({
      ...prev,
      errors: [],
    }))
  }, [])

  const handleDragEnter = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()
    setState((prev) => ({ ...prev, isDragging: true }))
  }, [])

  const handleDragLeave = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()

    if (e.currentTarget.contains(e.relatedTarget as Node)) {
      return
    }

    setState((prev) => ({ ...prev, isDragging: false }))
  }, [])

  const handleDragOver = useCallback((e: DragEvent<HTMLElement>) => {
    e.preventDefault()
    e.stopPropagation()
  }, [])

  // Simplified handleDrop function that delegates all folder processing to addFolders
  const handleDrop = useCallback(
      async (e: React.DragEvent<HTMLElement>) => {
        e.preventDefault()
        e.stopPropagation()
        setState((prev) => ({ ...prev, isDragging: false }))

        if (inputRef.current?.disabled && directoryInputRef.current?.disabled) {
          return
        }

        const items = e.dataTransfer.items

        // Handle modern browsers with webkitGetAsEntry support
        if (items && items.length > 0 && allowDirectories) {
          const entries = Array.from(items)
              .map(item => (item as any).webkitGetAsEntry?.())
              .filter(entry => entry)

          if (entries.length > 0) {
            const hasDirectories = entries.some(entry => entry.isDirectory)

            if (hasDirectories) {
              // Process directories using the enhanced traversal
              try {
                const allFiles: File[] = []

                for (const entry of entries) {
                  const files = await traverseDirectoryEntry(entry)
                  allFiles.push(...files)
                }

                if (allFiles.length > 0) {
                  await addFolders(allFiles)
                }
              } catch (error) {
                console.error('Error processing dropped directories:', error)
                setState((prev) => ({
                  ...prev,
                  errors: [...prev.errors, 'Error processing dropped folders.']
                }))
              }
              return
            }
          }
        }

        // Fallback for browsers without full drag/drop support or regular files
        if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
          const files = Array.from(e.dataTransfer.files)
          const hasRelativePaths = files.some((file) => !!(file as any).webkitRelativePath)

          if (hasRelativePaths && allowDirectories) {
            await addFolders(files)
          } else {
            // Handle as regular files
            if (!multiple) {
              addFiles([files[0]])
            } else {
              addFiles(files)
            }
          }
        }
      },
      [addFiles, addFolders, multiple, allowDirectories, traverseDirectoryEntry]
  )

  const handleFileChange = useCallback(
      (e: ChangeEvent<HTMLInputElement>) => {
        if (e.target.files && e.target.files.length > 0) {
          addFiles(e.target.files)
        }
      },
      [addFiles]
  )

  const handleFolderChange = useCallback(
      (e: ChangeEvent<HTMLInputElement>) => {
        if (e.target.files && e.target.files.length > 0) {
          addFolders(e.target.files)
        }
      },
      [addFolders]
  )

  const openFileDialog = useCallback(() => {
    if (inputRef.current) {
      inputRef.current.click()
    }
  }, [])

  const openDirectoryDialog = useCallback(() => {
    if (directoryInputRef.current && state.supportsDirectories) {
      directoryInputRef.current.click()
    }
  }, [state.supportsDirectories])

  const getInputProps = useCallback(
      (props: InputHTMLAttributes<HTMLInputElement> = {}) => {
        return {
          ...props,
          type: "file" as const,
          onChange: handleFileChange,
          accept: props.accept || accept,
          multiple: props.multiple !== undefined ? props.multiple : multiple,
          ref: inputRef,
        }
      },
      [accept, multiple, handleFileChange]
  )

  const getDirectoryInputProps = useCallback(
      (props: InputHTMLAttributes<HTMLInputElement> = {}) => {
        return {
          ...props,
          type: "file" as const,
          onChange: handleFolderChange,
          accept: props.accept || accept,
          multiple: true,
          webkitdirectory: 'true',
          ref: directoryInputRef,
        }
      },
      [accept, handleFolderChange]
  )

  return [
    state,
    {
      addFiles,
      addFolders,
      removeFile,
      removeFolder,
      clearFiles,
      clearFolders,
      clearAll,
      clearErrors,
      handleDragEnter,
      handleDragLeave,
      handleDragOver,
      handleDrop,
      handleFileChange,
      handleFolderChange,
      openFileDialog,
      openDirectoryDialog,
      getInputProps,
      getDirectoryInputProps,
    },
  ]
}

// Helper function to format bytes to human-readable format
export const formatBytes = (bytes: number, decimals = 2): string => {
  if (bytes === 0) return "0 Bytes"

  const k = 1024
  const dm = decimals < 0 ? 0 : decimals
  const sizes = ["Bytes", "KB", "MB", "GB", "TB", "PB", "EB", "ZB", "YB"]

  const i = Math.floor(Math.log(bytes) / Math.log(k))

  return Number.parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + sizes[i]
}
