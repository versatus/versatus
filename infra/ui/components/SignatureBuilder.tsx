// react component that builds out a Signature
import { useState } from 'react'
import TransactionBuilder from '@/components/TransactionBuilder'

export const FAKE_SIGNATURE = {
  timestamp: 1678756128,
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
  nonce: 0,
  private_key:
    'ba6ec9325d42dfde5ef2f24ea9f58dd23147e8604146c41fa8abc809c0ba3e21',
}

// React component that helps build and edit a signature state object
const SignatureBuilder = ({ signature }) => {
  const [sig, setSignature] = useState<any>(FAKE_SIGNATURE)

  return (
    <div className={''}>
      <TransactionBuilder signature={signature} tx={sig} />
    </div>
  )
}

export default SignatureBuilder
