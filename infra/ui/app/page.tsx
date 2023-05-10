'use client'
import Playground from '@/components/Playground'
import { NodeProvider } from '@/contexts/NodeProvider'

export default function Home() {
  return (
    <div>
      <NodeProvider>
        <Playground />
      </NodeProvider>
    </div>
  )
}
