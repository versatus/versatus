import Container from "@/components/Component";
import NodeOverview from "@/components/NodeOverview";
import AccountOverview from "@/components/AccountOverview";
import RPCMethodPreview from "@/components/RPCMethodPreview";
import {useNodeContext} from "@/contexts/NodeProvider";

const NodeInteraction = () => {
    const {address} = useNodeContext()
    return (
        <Container>
            <div className={'grid grid-cols-2 gap-4'}>
                <div className={'gap-4 flex-col flex'}>
                    <NodeOverview />
                    <AccountOverview />
                </div>
                <RPCMethodPreview address={address} />
            </div>
        </Container>
    )
}

export default NodeInteraction