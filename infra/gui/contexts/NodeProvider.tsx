import React, {createContext, ReactNode, useContext, useEffect, useState} from 'react'
import { getAccount, rpcFetcher } from '@/lib/methods'
import useSWR from 'swr'

interface NodeContextProps {
  fullState: any
  fullStateLoading: boolean | undefined
  fullStateErr: any
  memPool: any[]
  memPoolLoading: boolean | undefined
  memPoolErr: any
  nodeType: any
  nodeTypeLoading: boolean | undefined
  nodeTypeErr: any
  membershipConfig: any
  membershipConfigLoading: boolean | undefined
  membershipConfigErr: any
  lastBlock: any
  lastBlockLoading: boolean | undefined
  lastBlockErr: any
  transactionCount: number;
  transactionCountLoading: boolean | undefined;
  transactionCountErr: any;
  nodeHealth: any;
  nodeHealthLoading: boolean | undefined;
  nodeHealthErr: any;
  claimsByAccountId: any;
  claimsByAccountIdLoading: boolean | undefined;
  claimsByAccountIdErr: any;
  claimHashes: any[];
  claimHashesLoading: boolean | undefined;
  claimHashesErr: any;
  claims: any;
  claimsLoading: boolean | undefined;
  claimsErr: any;
  account: any
  address: string
  setAddress: (address: string) => void

}

const NodeContext = createContext<NodeContextProps>({
  account: undefined,
  address: '',
  setAddress: () => {},
  fullState: undefined,
  fullStateLoading: false,
  fullStateErr: undefined,
  memPool: [],
  memPoolLoading: false,
  memPoolErr: undefined,
  nodeType: undefined,
  nodeTypeLoading: false,
  nodeTypeErr: undefined,
  membershipConfig: undefined,
  membershipConfigLoading: false,
  membershipConfigErr: undefined,
  lastBlock: undefined,
  lastBlockLoading: false,
  lastBlockErr: undefined,
  transactionCount: 0,
  transactionCountLoading: false,
  transactionCountErr: undefined,
  nodeHealth: undefined,
  nodeHealthLoading: false,
  nodeHealthErr: undefined,
  claimsByAccountId: undefined,
  claimsByAccountIdLoading: false,
  claimsByAccountIdErr: undefined,
  claimHashes: [],
  claimHashesLoading: false,
  claimHashesErr: undefined,
  claims: undefined,
  claimsLoading: false,
  claimsErr: undefined,
} as NodeContextProps)

const useNodeContext = () => useContext(NodeContext)

const NodeProvider = ({ children }: {children: ReactNode}) => {
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

  const {
    data: membershipConfig,
    isLoading: membershipConfigLoading,
    error: membershipConfigErr,
  } = useSWR({ url: 'getMembershipConfig' }, rpcFetcher)

  const {
    data: lastBlock,
    isLoading: lastBlockLoading,
    error: lastBlockErr,
  } = useSWR({ url: 'getLastBlock' }, rpcFetcher)

  const {
    data: nodeHealth,
    isLoading: nodeHealthLoading,
    error: nodeHealthErr,
  } = useSWR({ url: 'getNodeHealth' }, rpcFetcher);

  const {
    data: claimHashes,
    isLoading: claimHashesLoading,
    error: claimHashesErr,
  } = useSWR({ url: 'getClaimHashes' }, rpcFetcher);



  //TODO: swrs need args under here
  const {
    data: transactionCount,
    isLoading: transactionCountLoading,
    error: transactionCountErr,
  } = useSWR({ url: 'getTransactionCount' }, rpcFetcher);

  const {
    data: claims,
    isLoading: claimsLoading,
    error: claimsErr,
  } = useSWR({ url: 'getClaims' }, rpcFetcher);

  const {
    data: claimsByAccountId,
    isLoading: claimsByAccountIdLoading,
    error: claimsByAccountIdErr,
  } = useSWR({ url: 'getClaimsByAccountId' }, rpcFetcher);




  useEffect(() => {
    if (fullState && Object.keys(fullState).length > 0) {
      setAddress(Object.keys(fullState)[0])
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
        membershipConfig,
        membershipConfigLoading,
        membershipConfigErr,
        lastBlock,
        lastBlockLoading,
        lastBlockErr,
        transactionCount,
        transactionCountLoading,
        transactionCountErr,
        nodeHealth,
        nodeHealthLoading,
        nodeHealthErr,
        claimsByAccountId,
        claimsByAccountIdLoading,
        claimsByAccountIdErr,
        claimHashes,
        claimHashesLoading,
        claimHashesErr,
        claims,
        claimsLoading,
        claimsErr,
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
