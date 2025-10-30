import {Window} from "@tauri-apps/api/window";
import {useEffect, useRef, useState} from "react";
import {getCurrentWindow, PhysicalPosition, PhysicalSize} from "@tauri-apps/api/window";
import {debounce} from "lodash";

export default function useWindow(onWindow: Window | undefined) {
    const window = onWindow || getCurrentWindow()
    const [size, setSize] = useState(new PhysicalSize(0, 0))
    const [position, setPosition] = useState(new PhysicalPosition(0, 0))
    const resultRef = useRef({
        size, position, window
    })

    useEffect(() => {
        resultRef.current.position = position
        resultRef.current.size = size
        resultRef.current.window = window
    }, [size, position, window]);

    const handleResize = useRef(
        debounce((event) => {
            const newSize = event.payload
            setSize(newSize)
        }, 240, { leading: false, trailing: true })
    ).current

    const handleMove = useRef(
        debounce((event) => {
            const newPos = event.payload
            setPosition(newPos)
        }, 240, { leading: false, trailing: true })
    ).current

    useEffect(() => {
        let unlistenResize: (() => void) | undefined
        let unlistenMove: (() => void) | undefined

        const setup = async () => {
            const [initSize, initPos] = await Promise.all([
                window.outerSize(),
                window.outerPosition(),
            ])
            setSize(initSize)
            setPosition(initPos)

            unlistenResize = await window.onResized(handleResize)
            unlistenMove = await window.onMoved(handleMove)
        }

        setup()

        return () => {
            unlistenResize?.()
            unlistenMove?.()
            handleResize.cancel()
            handleMove.cancel()
        }
    }, [])

    return resultRef.current
}