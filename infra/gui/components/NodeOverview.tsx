import StatsBlock from "@/components/StatsBlock";
import {useNodeContext} from "@/contexts/NodeProvider";

const NodeOverview = () => {
    const { memPool, nodeType } =
        useNodeContext()
    return (
        <div className={'flex flex-col gap-2 text-white '}>
            <div className={'text-gray-700 text-sm'}>Node Stats</div>
            <div className={'flex flex-row gap-3'}>
                <StatsBlock suffix={'txs in the mempool'}>
                    {memPool.length ?? 'N/A'}
                </StatsBlock>
                <StatsBlock suffix={'node type'}>{nodeType ?? 'N/A'}</StatsBlock>
            </div>
        </div>
    )
}

export default NodeOverview