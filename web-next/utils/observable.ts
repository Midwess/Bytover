type Listener<T> = (value: T) => void

export class Observable<T> {
    private value: T | undefined
    private listeners: Set<Listener<T | undefined>> = new Set()

    constructor(initialValue: T | undefined = undefined) {
        this.value = initialValue
    }

    subscribe(callback: Listener<T | undefined>): () => void {
        this.listeners.add(callback)

        callback(this.value)

        return () => {
            this.listeners.delete(callback)
        }
    }

    get(): T | undefined {
        return this.value
    }

    set(newValue: T): void {
        if (Object.is(this.value, newValue)) return // avoid unnecessary updates
        this.value = newValue
        this.listeners.forEach(listener => listener(this.value))
    }
}