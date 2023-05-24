import axios from 'axios'

export const getAccount = async (address: string) => {
  try {
    const response = await makeRPCCall('getAccount', [address])
    return response.data
  } catch (error) {
    throw error
  }
}

export const getFullMempool = async () => {
  try {
    const response = await makeRPCCall('getFullMempool')
    return response.data
  } catch (error) {
    throw error
  }
}

export const getNodeType = async () => {
  try {
    const response = await makeRPCCall('getNodeType')
    return response.data
  } catch (error) {
    throw error
  }
}

export const getFullState = async () => {
  try {
    const response = await makeRPCCall('getFullState')
    return response.data
  } catch (error) {
    throw error
  }
}

export const signTransaction = async (tx: {
  sender_address: string
  amount: number
  sender_public_key: string
  receiver_address: string
  private_key: string
  nonce: number
  timestamp: number
  token: { symbol: string; decimals: number; name: string }
}) => {
  try {
    const response = await makeRPCCall('signTransaction', [tx])
    return response.data
  } catch (error) {
    throw error
  }
}

export const createTransaction = async (tx: any) => {
  try {
    const response = await makeRPCCall('createTransaction', [tx])
    return response.data
  } catch (error) {
    throw error
  }
}

export const makeRPCCall = async (method: string, params = []) => {
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
    return await axios(config)
  } catch (error) {
    throw error
  }
}
