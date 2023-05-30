import React, { createContext, useContext, useEffect, useState } from 'react'
import {
  getAccount,
  getFullMempool,
  getFullState,
  getNodeType,
} from '@/lib/methods'

interface NodeContextProps {
  fullState: any
  memPool: any[]
  nodeType: string | null
  account: any

  address: string
  setAddress: (address: string) => void
}

const NodeContext = createContext<NodeContextProps>({})

const useNodeContext = () => useContext(NodeContext)

const NodeProvider: React.FC = ({ children }) => {
  const [fullState, setFullState] = useState<any>(null)
  const [memPool, setMemPool] = useState<any>([])
  const [nodeType, setNodeType] = useState<string | null>(null)
  const [account, setAccount] = useState<any>(null)
  const [address, setAddress] = useState<string>('')

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

  useEffect(() => {
    if (!address) return
    getAccount(address).then((res) => {
      setAccount(res.result)
    })
  }, [address])

  return (
    <NodeContext.Provider
      value={{ fullState, memPool, nodeType, account, address, setAddress }}
    >
      {children}
    </NodeContext.Provider>
  )
}

export { NodeProvider, useNodeContext }
