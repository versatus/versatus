import axios from 'axios'
import { NextResponse } from 'next/server'

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
    url: `http://${process.env["RPC_API_URL"]}`,
    headers: {
      'Content-Type': 'application/json',
    },
    data: data,
  }
  return await axios
    .request(config)
    .then((response) => {
      if (response.data.error) throw new Error(response.data.error.message)
      return NextResponse.json(response.data)
    })
    .catch((error) => {
      const respInit: ResponseInit = {
        status: 400,
      }
      const message = error.message
      return NextResponse.json(
        {
          message,
        },
        respInit
      )
    })
}
