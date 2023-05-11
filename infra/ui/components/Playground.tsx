'use client'
import { useState } from 'react'
import { FAKE_TRANSACTION } from '@/components/TransactionBuilder'
import { FAKE_SIGNATURE } from '@/components/SignatureBuilder'
import Container from '@/components/Component'
import { createTransaction, signTransaction } from '@/lib/methods'
import RPCMethodPreview from '@/components/RPCMethodPreview'
import { useNodeContext } from '@/contexts/NodeProvider'
import StatsBlock from '@/components/StatsBlock'
import { Box2 } from 'iconsax-react'

const Playground = () => {
  const { fullState, memPool, nodeType, account, address, setAddress } =
    useNodeContext()

  const TXBuilder = () => {
    const [signature, setSignature] = useState<string>('')

    const TXInputs = () => {
      const [senderAddress, setSenderAddress] = useState<string>(
        FAKE_TRANSACTION.sender_address
      )
      const [senderPublicKey, setSenderPublicKey] = useState<string>(
        FAKE_TRANSACTION.sender_public_key
      )
      const [receiverAddress, setReceiverAddress] = useState<string>(
        FAKE_TRANSACTION.receiver_address
      )
      const [amount, setAmount] = useState<number>(FAKE_TRANSACTION.amount)
      const [nonce, setNonce] = useState<number>(FAKE_TRANSACTION.nonce)

      return (
        <>
          <div className={'flex flex-col gap-1 w-full text-white'}>
            <label htmlFor="signature">Signature</label>
            <input
              readOnly
              onFocus={(e) => e.target.select()}
              type="text"
              name="signature"
              id="signature"
              placeholder="Signature"
              className="signature-input p-3 text-grey-400 border w-full rounded-xl"
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
                className="sender-address-input p-3 text-grey-400 border w-full text-[#000] rounded-xl"
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
                className="sender-public-key-input p-3 text-grey-400 border w-full rounded-xl"
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
                className="receiver-address-input p-3 text-grey-400 border w-full rounded-xl"
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
                className="amount-input p-3 text-grey-400 border w-full rounded-xl"
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
                className="nonce-input p-3 text-grey-400 border w-full rounded-xl"
                value={nonce}
                onChange={(e) => setNonce(parseInt(e.target.value))}
              />
            </div>
          </div>
        </>
      )
    }

    return (
      <div className={'flex flex-row gap-2'}>
        <div
          className={
            'text-sm w-[100%] text-white flex flex-col items-center justify-center  bg-blue-200 rounded-xl p-4 gap-3'
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
            onClick={() =>
              // @ts-ignore
              createTransaction({ ...FAKE_TRANSACTION, timestamp: Date.now() })
            }
            disabled={!signature}
            className="bg-gradient hover:bg-purple-500 border  text-white font-bold py-2 px-4 rounded-xl"
          >
            Send Transaction
          </button>
        </div>
      </div>
    )
  }

  const AccountSelectDropdown = () => {
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

  const AccountOverview = () => {
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

  const NodeOverview = () => {
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

  const NodeInteraction = () => {
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

  return (
    <div className={'flex flex-col gap-2'}>
      <div
        className={'gap-2 text-white flex flex-row rounded p-4 items-center'}
      >
        <Box2 />
        <div className={'text-md font-light text-white'}>
          Farmer Node Overview
        </div>
      </div>
      <div className={'flex flex-row gap-2'}></div>
      <div className={'border rounded-xl flex-flex-col gap-5 p-10'}>
        <div className={'flex flex-row items-center gap-2'}>
          <div className={'p-2 rounded-full bg-mars'}></div>
          <div className={' text-white'}>Reputation</div>
        </div>
        <div className={'flex flex-row items-center gap-2'}>
          <div className={'p-2 rounded-full bg-saturn'}></div>
          <div className={' text-white'}>Avg Reputation for Pool</div>
        </div>
        <div className={'flex flex-row items-center gap-2'}>
          <div className={'p-2 rounded-full bg-venus'}></div>
          <div className={' text-white'}>Avg Tx Processing Time</div>
        </div>
        <div className={'flex flex-row items-center gap-2'}>
          <div className={'p-2 rounded-full bg-neptune'}></div>
          <div className={' text-white'}>Total Votes</div>
        </div>
      </div>
      <div className={'flex flex-row gap-3 border rounded-xl p-10'}>
        <div
          className="radial-progress text-mars text-mars"
          style={{ '--value': 0 }}
        >
          0%
        </div>
        <div className="radial-progress text-saturn" style={{ '--value': 20 }}>
          20%
        </div>
        <div className="radial-progress text-mars" style={{ '--value': 60 }}>
          60%
        </div>
        <div className="radial-progress text-venus" style={{ '--value': 80 }}>
          80%
        </div>
        <div
          className="radial-progress text-neptune"
          style={{ '--value': 100 }}
        >
          100%
        </div>
        <div className="radial-progress text-venus" style={{ '--value': 80 }}>
          80%
        </div>
        <div className="radial-progress text-mars" style={{ '--value': 60 }}>
          60%
        </div>
        <div className="radial-progress text-saturn" style={{ '--value': 20 }}>
          20%
        </div>
        <div
          className="radial-progress text-mars text-mars"
          style={{ '--value': 0 }}
        >
          0%
        </div>

        <div className={'relative'}>
          <div
            className="radial-progress text-neptune"
            style={{
              '--value': 100,
              '--size': '12rem',
              '--thickness': '0.5rem',
            }}
          />
          <div
            className="radial-progress text-mars absolute left-[8.5%] top-[8.5%]"
            style={{
              '--value': 100,
              '--size': '10rem',
              '--thickness': '0.5rem',
            }}
          />
          <div
            className="radial-progress text-saturn absolute left-[17%] top-[17%]"
            style={{
              '--value': 100,
              '--size': '8rem',
              '--thickness': '0.5rem',
            }}
          />
          <div
            className="radial-progress text-venus absolute left-[25.33%] top-[25.33%]"
            style={{
              '--value': 100,
              '--size': '6rem',
              '--thickness': '0.5rem',
            }}
          />
          <div
            className="radial-progress text-earth absolute left-[33.5%] top-[33.5%]"
            style={{
              '--value': 100,
              '--size': '4rem',
              '--thickness': '0.5rem',
            }}
          />
        </div>
      </div>
      <NodeInteraction />
      <TXBuilder />
    </div>
  )
}

export default Playground
