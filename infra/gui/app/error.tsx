'use client' // Error components must be Client components

import { useEffect } from 'react'

export default function Error({
  error,
  reset,
}: {
  error: Error
  reset: () => void
}) {
  useEffect(() => {
    // Log the error to an error reporting service
    console.error(error)
  }, [error])

  return (
    <div className={'flex h-[100vh] w-[100vw] items-center justify-center'}>
      <div className={'flex flex-row gap-3 items-center'}>
        <h1 className={'text-[96px]'}>💀</h1>
        <button
          onClick={
            // Attempt to recover by trying to re-render the segment
            () => reset()
          }
        >
          Try again
        </button>
      </div>
    </div>
  )
}
