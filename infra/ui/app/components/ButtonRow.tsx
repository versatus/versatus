function ButtonRow(props) {
  const handleClick = (value) => {
    // Call function with the value of the clicked button
    props.onButtonClick(value)
  }

  return (
    <div className="flex flex-wrap gap-2 m-3 text-sm">
      {props.methods.map((method) => (
        <button
          key={method}
          onClick={() => handleClick(method)}
          className="bg-purple-500   color-gradient hover:bg-purple-700 border-4 border border-purple-500 text-white font-bold py-2 px-4 rounded"
        >
          {method}
        </button>
      ))}
    </div>
  )
}

export default ButtonRow
