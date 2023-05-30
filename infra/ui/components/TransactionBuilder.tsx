import { useState } from 'react'

export const FAKE_TRANSACTION = {
  timestamp: Date.now(),
  sender_address:
    '0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d4410a',
  sender_public_key:
    '031c0c705bee9901be2c221b71c490239b86d1518e1eeca9e9c0565f8da5e53797',
  receiver_address:
    '0351615b78ae431509ccf19f3d55e19e07baac0a4d024b999ff1c4234207d44106',
  token: {
    name: 'VRRB',
    symbol: 'VRRB',
    decimals: 18,
  },
  amount: 0,
  signature:
    '3045022100cfd569e53190fb9e01e6dfce8895049d953539c527862d818c2ac0dcf763bcf00220085bf1c74828121c21b0fb25621073408663891f9aca74c525421f963910b3ef',
  validators: {},
  nonce: 0,
  receiver_farmer_id: null,
}

const TransactionBuilder = ({ tx, signature }: any) => {
  const [senderAddress, setSenderAddress] = useState<string>(tx.sender_address)
  const [senderPublicKey, setSenderPublicKey] = useState<string>(
    tx.sender_public_key
  )
  const [receiverAddress, setReceiverAddress] = useState<string>(
    tx.receiver_address
  )
  const [amount, setAmount] = useState<number>(tx.amount)
  const [token, setToken] = useState<string>('VRRB')
  const [nonce, setNonce] = useState<number>(tx.nonce)

  return (
    <>
      <div className={'flex flex-col gap-1'}>
        <label htmlFor="signature">Signature</label>
        <input
          onFocus={(e) => e.target.select()}
          type="text"
          name="signature"
          id="signature"
          placeholder="Signature"
          className="signature-input p-3 text-black border w-full rounded-xl"
          value={signature}
        />
      </div>
      <div className="w-full flex flex-wrap text-sm p-4 gap-1">
        <div className={'flex flex-col gap-1'}>
          <label htmlFor="sender_address">Sender Address</label>
          <input
            onFocus={(e) => e.target.select()}
            type="text"
            name="sender_address"
            id="sender_address"
            placeholder="Sender Address"
            className="sender-address-input p-3 text-black border w-full text-[#000] rounded-xl"
            value={senderAddress}
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
          <label htmlFor="token">Token</label>
          <input
            onFocus={(e) => e.target.select()}
            type="text"
            name="token"
            id="token"
            placeholder="Token"
            className="token-input p-3 color-gradient text-black border w-full rounded-xl"
            value={token}
            onChange={(e) => setToken(e.target.value)}
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

export default TransactionBuilder
