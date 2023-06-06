import { Flash, Link, Share, User } from 'iconsax-react'
import { ButtonBadge } from '@vrrb/ui'
import { useNodeContext } from '@/contexts/NodeProvider'
import { ReactNode } from 'react'

export const NodeStatusModule = () => {
    const {
        fullState,
        fullStateLoading,
        fullStateErr,
        memPool,
        memPoolLoading,
        memPoolErr,
        nodeType,
        nodeTypeLoading,
        nodeTypeErr,
    } = useNodeContext()

    const LoadingButton = ({
        children,
        isLoading,
        isError,
        size = 'md',
        tooltip = null,
        className = '',
        disableAnimation = false,
    }: {
        children?: ReactNode
        isLoading?: boolean
        isError?: boolean
        size?: 'xs' | 'sm' | 'md' | 'lg' | 'xl'
        tooltip?: string | null
        className?: string
        disableAnimation?: boolean
    }) => {
        return (
            <div className="tooltip tooltip-left" data-tip={tooltip}>
                <div
                    className={`btn btn-${size} ${disableAnimation ? 'no-animation' : ''
                        } ${className} ${isError ? 'bg-error' : ''}`}
                >
                    {isLoading ? (
                        <span className={'animate-pulse'}>loading...</span>
                    ) : isError ? (
                        <span className={''}>error</span>
                    ) : (
                        children
                    )}
                </div>
            </div>
        )
    }

    return (
        <div className="border rounded-3xl text-[12px] p-10">
            <div className="justify-between text-white flex items-center h-full flex-col gap-6">
                <div className={'flex flex-row w-full items-center'}>
                    <div className={'flex flex-row gap-2 items-center'}>
                        <User />
                        Name Farmer
                    </div>
                    <div className={'grow'} />
                    <div className={'flex flex-row text-xs items-center gap-2'}>
                        <ButtonBadge
                            startIcon={
                                <div
                                    className={'p-3 rounded-full bg-neon-energy items-center'}
                                />
                            }
                            value={'0x18eb4ee1238gg1yg1g'}
                            className={'text-earth items-center'}
                        />
                        <ButtonBadge
                            className={'text-[#A2A2A2] h-full p-2'}
                            value={'confirmed'}
                        />
                    </div>
                </div>
                <div className={'flex flex-row w-full items-center'}>
                    <div className={'flex flex-row gap-2 items-center'}>
                        <Flash />
                        Full State
                    </div>
                    <div className={'grow'} />
                    <LoadingButton
                        size={'xs'}
                        isError={!!fullStateErr}
                        isLoading={fullStateLoading}
                        tooltip={'This is the current state of your node'}
                        className={'border-0'}
                        disableAnimation
                    >
                        {`${fullState ? 'synced' : 'not synced'}`}
                    </LoadingButton>
                </div>
                <div className={'flex flex-row w-full items-center'}>
                    <div className={'flex flex-row gap-2 items-center'}>
                        <Share />
                        Mempool
                    </div>
                    <div className={'grow'} />
                    <LoadingButton
                        size={'xs'}
                        isError={!!memPoolErr}
                        isLoading={memPoolLoading}
                        tooltip={'This is the number of transactions in the mempool'}
                        className={'border-0'}
                        disableAnimation
                    >
                        {`${memPool?.length ?? 0} Transactions`}
                    </LoadingButton>
                </div>
                <div className={'flex flex-row w-full items-center'}>
                    <div className={'flex flex-row gap-2 items-center'}>
                        <Link />
                        Type
                    </div>
                    <div className={'grow'} />
                    <LoadingButton
                        size={'xs'}
                        isError={!!nodeTypeErr}
                        isLoading={nodeTypeLoading}
                        tooltip={"This is the type of node you're running"}
                        className={'border-0'}
                        disableAnimation
                    >
                        {nodeType?.result}
                    </LoadingButton>
                </div>
            </div>
        </div>
    )
}
