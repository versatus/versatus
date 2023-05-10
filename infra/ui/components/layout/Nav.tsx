import VRRBLogo from '@vrrb/icons'

export default function Nav() {
  return (
    <nav className="">
      <div className=" flex flex-row flex-wrap items-center justify-between mx-auto p-2">
        <span className="self-center font-semibold whitespace-nowrap">
          <VRRBLogo />
        </span>
        <div>test</div>
      </div>
    </nav>
  )
}
