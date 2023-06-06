import { MaskLeft } from 'iconsax-react'
import {
  IconBadge,
  ButtonBadge,
  VRRBJuiceContainer,
  VRRBSolarIcon,
} from '@vrrb/ui'

const StakeModule = () => {
  return (
    <div className="border p-6 col-span-2 rounded-3xl grid grid-cols-2 grow flex flex-row text-white">
      <div className={'flex flex-col h-full grow'}>
        <div className={'text-white grow flex items-center'}>
          <div className={'flex flex-row items-center'}>
            <IconBadge>
              <VRRBSolarIcon />
            </IconBadge>
            <div className={'flex flex-col gap-1'}>
              <div className={'flex text-[#A2A2A2] text-[8px]'}>
                <ButtonBadge className={'p-0'} value={'VRRB PRICE'} />
              </div>
              <div className={'flex flex-row items-center gap-3'}>
                <div>$219 @ 0.06626 BTC</div>{' '}
                <span className={'text-[#BDFF51]'}>
                  <ButtonBadge className={'p-0'} value={'(+4.40%)'} />
                </span>
              </div>
            </div>
          </div>
        </div>
        <div className={'border-t mx-6'} />
        <div className={'text-white grow flex items-center '}>
          <div className={'flex flex-row items-center'}>
            <div
              className={
                'mx-4 relative bg-grey-400 rounded-3xl p-10 flex items-center justify-center'
              }
            >
              <div className={'absolute'}>
                <MaskLeft size="48" />
              </div>
            </div>
            <div className={'flex flex-col gap-1'}>
              <div className={'flex text-[#A2A2A2] text-[8px]'}>
                <ButtonBadge value={'STAKED AMOUNT'} />
              </div>
              <div className={'flex flex-row items-center gap-3'}>
                <div>$11,233.00</div>
                <span className={'text-[#BDFF51]'}>
                  <ButtonBadge value={'(+4.40%)'} />
                </span>
              </div>
            </div>
          </div>
        </div>
      </div>
      <div className={' border-l my-6 flex items-center'}>
        <div className={' flex flex-col items-center justify-center w-full'}>
          <VRRBJuiceContainer color={'green'} />
        </div>
      </div>
    </div>
  )
}

export default StakeModule
