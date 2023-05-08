'use client'
import { useEffect, useState } from 'react'
import axios from 'axios'

import { JsonViewer } from '@textea/json-viewer'
import ButtonRow from '@/components/ButtonRow'
import { FAKE_TRANSACTION } from '@/components/TransactionBuilder'
import { FAKE_SIGNATURE } from '@/components/SignatureBuilder'
import {toHex} from "@/utils/functions";
import {READ_METHODS} from "@/consts/system";
import Container from "@/components/Component";
import {createTransaction, getAccount, getFullMempool, getFullState, getNodeType, signTransaction} from "@/lib/methods";

const Playground = () => {
  const [tx, setTx] = useState<any>(FAKE_TRANSACTION)
  const [data, setData] = useState(null)
  const [currentMethod, setCurrentMethod] = useState<string | null>(null)
  const [fullState, setFullState] = useState<any>(null)
  const [memPool, setMemPool] = useState<any>([])
  const [nodeType, setNodeType] = useState<string | null>(null)
  const [account, setAccount] = useState<any>(null)
  const [address, setAddress] = useState<string>('')
  const [signature, setSignature] = useState<string>("")
  const [senderAddress, setSenderAddress] = useState<string>(tx.sender_address)
  const [senderPublicKey, setSenderPublicKey] = useState<string>(
    tx.sender_public_key
  )
  const [receiverAddress, setReceiverAddress] = useState<string>(
    tx.receiver_address
  )
  const [amount, setAmount] = useState<number>(tx.amount)
  const [nonce, setNonce] = useState<number>(tx.nonce)


  useEffect(() => {
    if (!address) return
    getAccount(address).then((res) => {
        setAccount(res.result)
    })
  }, [address])

  useEffect(() => {
    getFullMempool().then((res) => {
        setMemPool(res.result)
    })
    getNodeType().then((res) => {
        setNodeType(res.result)
    })
    getFullState().then((res) => {
        setFullState(res.result)
    })
  }, [])

  useEffect(() => {
    if (fullState && Object.keys(fullState).length > 0) {
      setAddress(Object.keys(fullState ?? {})?.[0])
    }
  }, [fullState])


  const makeRPCCall = async (method: string, params = []) => {
    setCurrentMethod(method)
    const config = {
      method: 'post',
      maxBodyLength: Infinity,
      url: '/rpc',
      headers: {
        'Content-Type': 'application/json',
      },
      data: {
        method: `state_${method}`,
        params: params,
      },
    }

    try {
      const response = await axios(config)
      setData(response.data.result ?? response.data.error)
      if (method === 'getFullMempool') {
        setMemPool(response.data.result)
      } else if (method === 'getNodeType') {
        setNodeType(response.data.result)
      } else if (method === 'getAccount') {
        setAccount(response.data.result)
      } else if (method === 'getFullState') {
        setFullState(response.data.result)
      } else if (method === 'signTransaction') {
        console.log(response.data.result)
        setSignature(response.data.result)
      }
    } catch (error) {
      console.log(error)
    }
  }

  const selectAddress = (address) => {
    setAddress(address)
    const arr = new Uint8Array(fullState[address].pubkey)
    setSenderPublicKey(toHex(arr))
  }


  // react component that returns a map of Buttons based on array of methods
  const StatsBlock = ({ children, suffix }) => {
    return (
      <div>
        <div
          className={
            'flex flex-col flex-wrap border rounded-t-xl border-4 p-4 align-end '
          }
        >
          <span className={'color-gradient'}>{children}</span>{' '}
          <span className={'italic text-xs text-gray-600'}>{suffix}</span>
        </div>
      </div>
    )
  }

  const TXInputs = () => {
    return (
        <>
          <div className={'flex flex-col gap-1 w-full'}>
            <label htmlFor="signature">Signature</label>
            <input
                readOnly
                onFocus={(e) => e.target.select()}
                type="text"
                name="signature"
                id="signature"
                placeholder="Signature"
                className="signature-input p-3 text-black border w-full rounded-xl"
                value={signature}
            />
          </div>
          <div className="w-full flex flex-wrap text-sm p-4 gap-1 justify-center">
            <div className={'flex flex-col gap-1'}>
              <label htmlFor="sender_address">Sender Address</label>
              <input
                  onFocus={(e) => e.target.select()}
                  type="text"
                  name="sender_address"
                  id="sender_address"
                  placeholder="Sender Address"
                  className="sender-address-input p-3 text-black border w-full text-[#000] rounded-xl"
                  value={String(address)}
                  onChange={(e) => setSenderAddress(e.target.value)}
              />
            </div>
            <div className={'flex flex-col gap-1'}>
              <label htmlFor="sender_public_key">Sender Public Key</label>
              <input
                  onFocus={(e) => e.target.select()}
                  type="text"
                  name="sender_public_key"
                  id="sender_public_key"
                  placeholder="Sender Public Key"
                  className="sender-public-key-input p-3 text-black border w-full rounded-xl"
                  value={senderPublicKey}
                  onChange={(e) => setSenderPublicKey(e.target.value)}
              />
            </div>
            <div className={'flex flex-col gap-1'}>
              <label htmlFor="receiver_address">Receiver Address</label>
              <input
                  onFocus={(e) => e.target.select()}
                  type="text"
                  name="receiver_address"
                  id="receiver_address"
                  placeholder="Receiver Address"
                  className="receiver-address-input p-3 text-black border w-full rounded-xl"
                  value={receiverAddress}
                  onChange={(e) => setReceiverAddress(e.target.value)}
              />
            </div>
            <div className={'flex flex-col gap-1'}>
              <label htmlFor="amount">Amount</label>
              <input
                  onFocus={(e) => e.target.select()}
                  type="text"
                  name="amount"
                  id="amount"
                  placeholder="Amount"
                  className="amount-input p-3 text-black border w-full rounded-xl"
                  value={amount}
                  onChange={(e) => setAmount(parseInt(e.target.value))}
              />
            </div>
            <div className={'flex flex-col gap-1'}>
              <label htmlFor="nonce">Nonce</label>
              <input
                  onFocus={(e) => e.target.select()}
                  type="text"
                  name="nonce"
                  id="nonce"
                  placeholder="Nonce"
                  className="nonce-input p-3 text-black border w-full rounded-xl"
                  value={nonce}
                  onChange={(e) => setNonce(parseInt(e.target.value))}
              />
            </div>
          </div>
        </>
    )
  }

  const TXBuilder = () => {
    return (
        <div className={'flex flex-row gap-2'}>
          <div
              className={
                'text-sm w-[100%] flex flex-col items-center justify-center  bg-blue-200 rounded-xl p-4 gap-3'
              }
          >
            <TXInputs />
            <button
                onClick={() => signTransaction(FAKE_SIGNATURE)}
                className="bg-gradient hover:bg-purple-500 border-4 border border-purple-500 text-white font-bold py-2 px-4 rounded-xl"
            >
              Sign Transaction
            </button>
            <button
                onClick={() => createTransaction({ ...tx, timestamp: Date.now() })}
                disabled={!signature}
                className="bg-gradient hover:bg-purple-500 border-4 border border-purple-500 text-white font-bold py-2 px-4 rounded-xl"
            >
              Send Transaction
            </button>
          </div>
        </div>
    )
  }

  const AccountOverview = () => {
    return (
        <div className={'flex flex-col gap-2'}>
          <div className={'text-gray-700 text-sm gap-2 flex items-center'}>
            Account Overview
            <select
                className="select select-sm w-full max-w-xs"
                onChange={(e) => selectAddress(e.target.value)}
                value={String(address)}
            >
              <option disabled>Pick an account</option>
              {fullState &&
                  Object.keys(fullState).map((key) => {
                    return <option key={key}>{key}</option>
                  })}
            </select>
          </div>
          <div className={'flex flex-row gap-3'}>
            <StatsBlock suffix={'debits'}>
              {account?.debits ?? 'N/A'}
            </StatsBlock>
            <StatsBlock suffix={'credits'}>
              {account?.credits ?? 'N/A'}
            </StatsBlock>
            <StatsBlock suffix={'storage'}>
              {account?.storage ?? 'N/A'}
            </StatsBlock>
            <StatsBlock suffix={'code'}>
              {account?.code ?? 'N/A'}
            </StatsBlock>
            <StatsBlock suffix={'nonce'}>
              {account?.nonce ?? 'N/A'}
            </StatsBlock>
          </div>
        </div>
    )
  }

  const NodeOverview = () => {
    return (
        <div className={'flex flex-col gap-2'}>
          <div className={'text-gray-700 text-sm'}>Node Stats</div>
          <div className={'flex flex-row gap-3'}>
            <StatsBlock suffix={"txs in the mempool"}>
              {memPool.length ?? 'N/A'}
            </StatsBlock>
            <StatsBlock suffix={'node type'}>
              {nodeType ?? 'N/A'}
            </StatsBlock>
          </div>
        </div>
    )
  }

  const RPCMethodPreview = () => {
    return (
        <div>
          <ButtonRow
              methods={READ_METHODS}
              onButtonClick={(value) => makeRPCCall(value, [])}
          />
          <div
              className={
                'text-xs border p-4 border-4 rounded-xl h-[200px] overflow-scroll'
              }
          >
            <JsonViewer defaultInspectDepth={1} value={data} />
          </div>
        </div>
    )
  }

  const NodeInteraction = () => {
    return (
        <Container>
          <div className={'grid grid-cols-2 gap-4'}>
            <div className={'gap-4 flex-col flex'}>
              <NodeOverview />
              <AccountOverview />
            </div>
            <RPCMethodPreview />
          </div>
        </Container>
    )
  }

  return (
    <div className={'m-4 flex flex-col gap-2 '}>
      <NodeInteraction />
      <TXBuilder />
    </div>
  )
}

export default Playground
