import axios from 'axios'
import { NextResponse } from 'next/server'

const RPC_URL = 'http://127.0.0.1:9293'

export async function POST(request: Request) {
  const { method, params = [] } = await request.json()

  const data = JSON.stringify({
    id: 1,
    jsonrpc: '2.0',
    method,
    params,
  })

  const config = {
    method: 'post',
    maxBodyLength: Infinity,
    url: RPC_URL,
    headers: {
      'Content-Type': 'application/json',
    },
    data: data,
  }

  const resp = await axios
    .request(config)
    .then((response) => {
      return response.data
    })
    .catch((error) => {
      console.log(error)
    })
  return NextResponse.json(resp)
}
