import StatsBlock from "@/components/StatsBlock";
import AccountSelectDropdown from "@/components/AccountSelectDropdown";
import {useNodeContext} from "@/contexts/NodeProvider";

const AccountOverview = () => {
    const {  account } =
        useNodeContext()
    return (
        <div className={'flex flex-col gap-2 text-white'}>
            <AccountSelectDropdown />
            <div className={'flex flex-row gap-3'}>
                <StatsBlock suffix={'debits'}>{account?.debits ?? 'N/A'}</StatsBlock>
                <StatsBlock suffix={'credits'}>
                    {account?.credits ?? 'N/A'}
                </StatsBlock>
                <StatsBlock suffix={'storage'}>
                    {account?.storage ?? 'N/A'}
                </StatsBlock>
                <StatsBlock suffix={'code'}>{account?.code ?? 'N/A'}</StatsBlock>
                <StatsBlock suffix={'nonce'}>{account?.nonce ?? 'N/A'}</StatsBlock>
            </div>
        </div>
    )
}

export default AccountOverview