import { useEffect, useRef } from 'react'

const FAVICON_SIZE = 32
const PROGRESS_LINE_WIDTH = 3
const PROGRESS_COLOR = '#22c55e'
const PROGRESS_BG_COLOR = 'rgba(150, 150, 150, 0.4)'

function getAllFavicons(): HTMLLinkElement[] {
    return Array.from(document.querySelectorAll<HTMLLinkElement>('link[rel="icon"], link[rel="shortcut icon"]'))
}

function resolveBaseFaviconHref(originalHrefs: Map<HTMLLinkElement, string>): string | null {
    const favicons = getAllFavicons()
    if (favicons.length === 0) return null

    const getHref = (fav: HTMLLinkElement) => originalHrefs.get(fav) ?? fav.href

    const mediaMatched = favicons.find(fav => fav.media && window.matchMedia(fav.media).matches)
    if (mediaMatched) {
        return getHref(mediaMatched)
    }

    const plainIcon = favicons.find(fav => !fav.media)
    if (plainIcon) {
        return getHref(plainIcon)
    }

    return getHref(favicons[0])
}

export function useFaviconProgress(progress: number | null) {
    const canvasRef = useRef<HTMLCanvasElement | null>(null)
    const originalHrefsRef = useRef<Map<HTMLLinkElement, string>>(new Map())
    const faviconImageRef = useRef<HTMLImageElement | null>(null)

    // Initialize canvas and load favicon image on mount
    useEffect(() => {
        if (typeof window === 'undefined') return

        canvasRef.current = document.createElement('canvas')
        canvasRef.current.width = FAVICON_SIZE
        canvasRef.current.height = FAVICON_SIZE

        // Store original hrefs for all favicons
        getAllFavicons().forEach(fav => {
            originalHrefsRef.current.set(fav, fav.href)
        })

        faviconImageRef.current = new Image()
        faviconImageRef.current.crossOrigin = 'anonymous'

        return () => {
            // Restore all original favicons on unmount
            originalHrefsRef.current.forEach((href, fav) => {
                fav.href = href
            })
        }
    }, [])

    // Update favicon when progress changes
    useEffect(() => {
        if (typeof window === 'undefined') return

        const allFavicons = getAllFavicons()
        const canvas = canvasRef.current
        if (!canvas) return

        const ctx = canvas.getContext('2d')
        if (!ctx) return

        const shouldShowProgress = progress !== null && progress > 0 && progress < 1

        if (!shouldShowProgress) {
            // Restore all original favicons
            originalHrefsRef.current.forEach((href, fav) => {
                fav.href = href
            })
            return
        }

        const baseFaviconHref = resolveBaseFaviconHref(originalHrefsRef.current)
        if (!baseFaviconHref) return

        const drawProgress = () => {
            ctx.clearRect(0, 0, FAVICON_SIZE, FAVICON_SIZE)

            // Draw favicon image in center (smaller to make room for circle)
            const img = faviconImageRef.current
            if (img?.complete && img.naturalWidth > 0) {
                const padding = PROGRESS_LINE_WIDTH + 2
                const imgSize = FAVICON_SIZE - padding * 2
                ctx.drawImage(img, padding, padding, imgSize, imgSize)
            }

            const centerX = FAVICON_SIZE / 2
            const centerY = FAVICON_SIZE / 2
            const radius = (FAVICON_SIZE - PROGRESS_LINE_WIDTH) / 2

            // Background circle track
            ctx.beginPath()
            ctx.arc(centerX, centerY, radius, 0, 2 * Math.PI)
            ctx.strokeStyle = PROGRESS_BG_COLOR
            ctx.lineWidth = PROGRESS_LINE_WIDTH
            ctx.stroke()

            // Progress arc
            const startAngle = -Math.PI / 2
            const endAngle = startAngle + (2 * Math.PI * progress)

            ctx.beginPath()
            ctx.arc(centerX, centerY, radius, startAngle, endAngle)
            ctx.strokeStyle = PROGRESS_COLOR
            ctx.lineWidth = PROGRESS_LINE_WIDTH
            ctx.lineCap = 'round'
            ctx.stroke()

            const dataUrl = canvas.toDataURL('image/png')

            // Set ALL favicons to the progress image
            allFavicons.forEach(fav => {
                fav.href = dataUrl
            })
        }

        // If image is loaded, draw immediately, otherwise wait for it
        const img = faviconImageRef.current
        if (!img) return

        const onLoad = () => {
            drawProgress()
        }

        if (img.src !== baseFaviconHref) {
            img.src = baseFaviconHref
        }

        if (img.complete && img.naturalWidth > 0) {
            drawProgress()
            return
        }

        img.addEventListener('load', onLoad)
        return () => {
            img.removeEventListener('load', onLoad)
        }
    }, [progress])
}
