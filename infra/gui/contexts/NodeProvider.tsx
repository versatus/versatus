import React, { createContext, useContext, useEffect, useState } from 'react'
import { getAccount, rpcFetcher } from '@/lib/methods'
import useSWR from 'swr'

interface NodeContextProps {
  fullState: any
  fullStateLoading: boolean
  fullStateErr: any
  memPool: any[]
  memPoolLoading: boolean
  memPoolErr: any
  nodeType: string | null
  nodeTypeLoading: boolean
  nodeTypeErr: any
  account: any
  address: string
  setAddress: (address: string) => void
}

const NodeContext = createContext<NodeContextProps>({
  account: undefined,
  address: '',
  fullState: undefined,
  fullStateLoading: false,
  fullStateErr: undefined,
  memPool: [],
  memPoolLoading: false,
  memPoolErr: undefined,
  nodeType: undefined,
  nodeTypeLoading: false,
  nodeTypeErr: undefined,
} as NodeContextProps)

const useNodeContext = () => useContext(NodeContext)

const NodeProvider: React.FC = ({ children }) => {
  const [account, setAccount] = useState<any>(null)
  const [address, setAddress] = useState<string>('')

  const {
    data: fullState,
    isLoading: fullStateLoading,
    error: fullStateErr,
  } = useSWR({ url: 'getFullState' }, rpcFetcher)

  const {
    data: memPool,
    isLoading: memPoolLoading,
    error: memPoolErr,
  } = useSWR({ url: 'getFullMempool' }, rpcFetcher)

  const {
    data: nodeType,
    isLoading: nodeTypeLoading,
    error: nodeTypeErr,
  } = useSWR({ url: 'getNodeType' }, rpcFetcher)

  useEffect(() => {
    if (fullState && Object.keys(fullState).length > 0) {
      setAddress(Object.keys(fullState ?? {})?.[0])
    }
  }, [fullState])

  console.log('fullState', fullState)

  useEffect(() => {
    getAccount(
      '03f09c3ee3c5467ca11957395dd35fe60d004f8b664d51b3e720ca121f56b030f5'
    ).then((res) => {
      console.log('stuff', res)
      // setAccount(res.result)
    })
  }, [address])

  return (
    <NodeContext.Provider
      value={{
        fullState,
        fullStateLoading,
        fullStateErr,
        memPool,
        memPoolLoading,
        memPoolErr,
        nodeType,
        nodeTypeLoading,
        nodeTypeErr,
        account,
        address,
        setAddress,
      }}
    >
      {children}
    </NodeContext.Provider>
  )
}

export { NodeProvider, useNodeContext }
