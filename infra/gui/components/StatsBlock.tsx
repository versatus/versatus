// react component that returns a map of Buttons based on array of methods
const StatsBlock = ({ children, suffix }) => {
  return (
    <div>
      <div
        className={'flex flex-col flex-wrap border rounded-t-xl p-4 align-end '}
      >
        <span
          className={
            'text-2xl bg-clip-text text-transparent bg-gradient-to-r from-mars to-saturn '
          }
        >
          {children}
        </span>{' '}
        <span className={'italic text-xs text-gray-600 text-primary'}>
          {suffix}
        </span>
      </div>
    </div>
  )
}

export default StatsBlock
