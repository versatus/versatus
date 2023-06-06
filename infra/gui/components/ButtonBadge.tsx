const ButtonBadge = ({ value }: { value: string }) => {
  return (
    <div className={`cursor-default rounded-md bg-[#1C1C1C] p-1 text-xs`}>
      {value}
    </div>
  )
}

export default ButtonBadge
