import {useNodeContext} from "@/contexts/NodeProvider";

const AccountSelectDropdown = () => {
    const { fullState, address, setAddress } =
        useNodeContext()
    return (
        <div className={'text-sm gap-2 flex items-center'}>
            Account Overview
            <select
                className="select select-sm w-full max-w-xs"
                onChange={(e) => setAddress(e.target.value)}
                value={String(address)}
            >
                <option disabled>Pick an account</option>
                {fullState &&
                    Object.keys(fullState).map((key) => {
                        return <option key={key}>{key}</option>
                    })}
            </select>
        </div>
    )
}


export default AccountSelectDropdown