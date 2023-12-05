'use client'
import { NodeProvider } from '@/contexts/NodeProvider'
import { NodeStatusModule } from '@/components/modules/NodeStatusModule'
import { Wind2 } from 'iconsax-react'

export default function Home() {
  const PageHeader = () => {
    return (
      <div
        className={
          'gap-2 text-white flex flex-row rounded p-0 m-0 items-center'
        }
      >
        <Wind2 />
        <div className={'text-md font-light text-white'}>
          Farmer Node Overview
        </div>
      </div>
    )
  }
  return (
    <div>
      <NodeProvider>
        <div className={'flex flex-col gap-3  mx-4'}>
          <PageHeader />
          <div className={'grid grid-cols-1 lg:grid-cols-3 gap-8'}>
            <NodeStatusModule />
          </div>
        </div>
      </NodeProvider>
    </div>
  )
}
