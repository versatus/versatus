'use client'
import { NodeProvider } from '@/contexts/NodeProvider'
import ReputationModule from '@/components/modules/ReputationModule'
import GainsModule from '@/components/modules/GainsModule'
import { NodeStatusModule } from '@/components/modules/NodeStatusModule'
import { Wind2 } from 'iconsax-react'
import RPCMethodPreview from '@/components/RPCMethodPreview'

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
            <ReputationModule />
            <GainsModule />
            <NodeStatusModule />
            <RPCMethodPreview address={''} />
          </div>
        </div>
      </NodeProvider>
    </div>
  )
}
