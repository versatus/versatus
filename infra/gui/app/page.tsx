'use client'
import {NodeProvider, useNodeContext} from '@/contexts/NodeProvider'
import { NodeStatusModule } from '@/components/modules/NodeStatusModule'
import { Wind2 } from 'iconsax-react'
import {ReactNode} from "react";

export default function Home() {


  return (
    <div>
      <NodeProvider>
        <AppContainer />
      </NodeProvider>
    </div>
  )
}

const AppContainer = () => {

  return (
      <div className={'flex flex-col gap-3  mx-4'}>
        <PageHeader />
        <div className={'grid grid-cols-1 lg:grid-cols-2 gap-8'}>
          <NodeStatusModule />
        </div>
      </div>
  )
}

const PageHeader = () => {
  const {nodeType} = useNodeContext()
  return (
      <div
          className={
            'gap-2 text-white flex flex-row rounded p-0 m-0 items-center'
          }
      >
        <Wind2 />
        <div className={'text-md font-light text-white'}>
          {nodeType?.result.slice(0,1).toUpperCase() + nodeType?.result.slice(1, nodeType?.result.length).toLowerCase()} Node Overview
        </div>
      </div>
  )
}