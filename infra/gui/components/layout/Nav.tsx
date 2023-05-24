import VRRBLogo from '@vrrb/icons'
import { ArrowDown2, Box1, HambergerMenu, SearchNormal } from 'iconsax-react'

export default function Nav() {
  return (
    <div className="navbar items-center">
      <div className="navbar-start">
        <a className="btn btn-ghost normal-case text-xl">
          <VRRBLogo />
        </a>
        <div className={'text-white flex flex-row items-center gap-2 text-sm'}>
          <Box1 size="16" />
          <div>Playground</div>
          <ArrowDown2 size="16" />
        </div>
      </div>
      <div className="navbar-end">
        <button className="text-white btn-circle text-xl">
          <SearchNormal size="24" />
        </button>
        <button className="text-white btn-circle text-xl">
          <HambergerMenu size="24" />
        </button>
      </div>
    </div>
  )
}
