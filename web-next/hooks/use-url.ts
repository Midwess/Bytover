// app/hooks/useUrlState.ts
'use client' // IMPORTANT: Required for useRouter() to work

import { useRouter, useSearchParams } from 'next/navigation'
import { useCallback, useMemo } from 'react'

/**
 * Custom hook to sync URL query params with component state in Next.js App Router.
 * @param paramNames - List of query parameter names to track.
 */
export function useUrlState(paramNames: string[]): [
    Record<string, string | null>,
    (newState: Record<string, string | null | undefined>) => void
] {
    const router = useRouter()
    const searchParams = useSearchParams()

    const state = useMemo(() => {
        return paramNames.reduce<Record<string, string | null>>((acc, paramName) => {
            const value = searchParams.get(paramName)
            acc[paramName] = value
            return acc
        }, {})
    }, [searchParams, paramNames])

    const setState = useCallback(
        (newState: Record<string, string | null | undefined>) => {
            const params = new URLSearchParams(searchParams.toString())

            // Remove existing tracked keys
            paramNames.forEach((key) => params.delete(key))

            // Add updated keys
            Object.entries(newState).forEach(([key, value]) => {
                if (value != null && value !== undefined) {
                    params.set(key, value)
                }
            })

            params.sort()

            // Replace URL with updated query string
            router.replace(`?${params.toString()}`)
        },
        [router, searchParams, paramNames]
    )

    return [state, setState]
}