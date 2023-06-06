import { useState } from 'react'
import {
  getAccount,
  getFullMempool,
  getFullState,
  getNodeType,
} from '@/lib/methods'
import { JsonViewer } from '@textea/json-viewer'

const RPCMethodPreview = ({ address }: { address: string }) => {

  const [rpcResp, setRpcResp] = useState<any>(null)
  const [method, setMethod] = useState<string>('')
  const getMemPoolTest = async () => {
    setMethod('getFullMempool')
    setRpcResp(await getFullMempool())
  }

  const getNodeTypeTest = async () => {
    setMethod('getNodeType')
    setRpcResp(await getNodeType())
  }

  const getFullStateTest = async () => {
    setMethod('getFullState')
    setRpcResp(await getFullState())
  }

  const getAccountTest = async () => {
    setMethod('getAccount')
    setRpcResp(await getAccount(address))
  }

  return (
    <div>
      <div className={'flex flex-row gap-2 m-2'}>
        <button
          disabled={method === 'getFullMempool'}
          onClick={getMemPoolTest}
          className="btn no-animation"
        >
          Get Mempool
        </button>
        <button
          disabled={method === 'getNodeType'}
          onClick={getNodeTypeTest}
          className="btn no-animation"
        >
          Get Node Type
        </button>
        <button
          disabled={method === 'getFullState'}
          onClick={getFullStateTest}
          className="btn no-animation"
        >
          Get Node Type
        </button>
        <button
          disabled={method === 'getAccount'}
          onClick={getAccountTest}
          className="btn no-animation"
        >
          Get Account
        </button>
      </div>
      <div
        className={'text-xs border p-4 text-white h-[200px] overflow-scroll'}
      >
        {rpcResp ? (
          <JsonViewer defaultInspectDepth={1} value={rpcResp} />
        ) : (
          'Click on a button to see the response'
        )}
      </div>
    </div>
  )
}

export default RPCMethodPreview
