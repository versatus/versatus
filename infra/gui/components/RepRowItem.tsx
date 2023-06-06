import { ArrowCircleUp2, ArrowCircleDown2 } from 'iconsax-react'

const RepRowItem = ({
  title,
  btnText,
  pctChange,
  displayColor,
}: {
  title: string
  btnText: string
  displayColor?: string
  pctChange?: number
}) => {
  const dynamicColor =
    pctChange > 0
      ? `bg-neon-energy bg-clip-text text-transparent`
      : pctChange === 0
      ? 'text-white'
      : 'bg-neon-solar bg-clip-text text-transparent'
  return (
    <div className=" tracking-wide text-sm text-white text-sm w-full bg- flex flex-row gap-2 items-center">
      <div className={`${displayColor} p-2 rounded-full`} />
      <p>{title}</p>
      <div
        className={`flex flex-row gap-2 text-xs items-center ${dynamicColor}`}
      >
        {pctChange > 0 ? (
          <ArrowCircleUp2 className={`text-xs text-earth h-4 w-4`} />
        ) : (
          <ArrowCircleDown2 className={`text-mars h-4 w-4`} />
        )}
        <div className={`${dynamicColor} -ml-1 text-[9px]`}>
          %{pctChange.toFixed(1)}
        </div>
      </div>
      <div className={'grow'} />
      <div className={'btn btn-xs bg-grey-400 border-0 font-semibold text-xs'}>
        {btnText}
      </div>
    </div>
  )
}

export default RepRowItem
