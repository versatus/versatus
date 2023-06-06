'use client'
import { Wind2 } from 'iconsax-react'
import ReputationModule from '@/components/modules/ReputationModule'
import GainsModule from '@/components/modules/GainsModule'
import { NodeStatusModule } from '@/components/modules/NodeStatusModule'

const Playground = () => {
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
        <div className={'flex flex-col gap-3'}>
            <PageHeader />
            <div className={'grid grid-cols-1 md:grid-cols-3 gap-8 mx-4'}>
                <ReputationModule />
                <GainsModule />
                <NodeStatusModule />
            </div>
        </div>
    )
}

export default Playground
