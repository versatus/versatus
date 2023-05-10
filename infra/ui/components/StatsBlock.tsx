// react component that returns a map of Buttons based on array of methods
const StatsBlock = ({ children, suffix }) => {
  return (
    <div>
      <div
        className={
          'flex flex-col flex-wrap border rounded-t-xl border-4 p-4 align-end '
        }
      >
        <span className={'color-gradient'}>{children}</span>{' '}
        <span className={'italic text-xs text-gray-600'}>{suffix}</span>
      </div>
    </div>
  )
}

export default StatsBlock
