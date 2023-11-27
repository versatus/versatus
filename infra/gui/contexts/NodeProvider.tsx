import React, {createContext, ReactNode, useContext, useEffect, useState} from 'react'
import { getAccount, rpcFetcher } from '@/lib/methods'
import useSWR from 'swr'

interface NodeContextProps {
  fullState: any
  fullStateLoading: boolean
  fullStateErr: any
  memPool: any[]
  memPoolLoading: boolean
  memPoolErr: any
  nodeType: string | null | undefined
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
  setAddress: (address: string) => {},
} as NodeContextProps)

const useNodeContext = () => useContext(NodeContext)

const NodeProvider  = ({ children }: {children: ReactNode}) => {
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

  useEffect(() => {
    getAccount(
      '0xae903d06d636f451eb6c5189e453c38fd7b7d694'
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
