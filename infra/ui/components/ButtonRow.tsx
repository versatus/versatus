import {getAccount, getFullMempool, getFullState, getNodeType} from "@/lib/methods";

function ButtonRow(props) {
  const methodObject = {
    getAccount: getAccount,
    getFullMempool: getFullMempool,
    getNodeType: getNodeType,
    getFullState: getFullState,
  }

  return (
    <div className="flex flex-wrap gap-2 m-3 text-sm">
      {props.methods.map((method) => (
        <button
          key={method}
          onClick={() => methodObject[method]()}
          className="bg-purple-500   color-gradient hover:bg-purple-700 border-4 border border-purple-500 text-white font-bold py-2 px-4 rounded"
        >
          {method}
        </button>
      ))}
    </div>
  )
}

export default ButtonRow
