import { Clock } from 'iconsax-react'
import {
  VRRBSolarIcon,
  IconBadge,
  ButtonBadge,
  PercentChangedBadge,
} from '@vrrb/ui'

const GainsModule = () => {
  return (
    <div className="border p-6 rounded-3xl flex flex-row text-white">
      <div className={'flex flex-col h-full grow gap-3'}>
        <div className={'text-white grow flex items-center '}>
          <div className={'flex flex-row items-center'}>
            <IconBadge>
              <VRRBSolarIcon />
            </IconBadge>
            <div className={'flex flex-col gap-1'}>
              <div className={'flex text-[#A2A2A2] text-[8px]'}>
                <ButtonBadge className={'p-0'} value={'Gains Made'} />
              </div>
              <div className={'flex flex-row items-center gap-3'}>
                <div>$66,433</div>{' '}
                <span className={'text-[#BDFF51]'}>
                  <PercentChangedBadge value={4.2} />
                </span>
              </div>
            </div>
          </div>
        </div>
        <div className={'border-t mx-6'} />
        <div className={'text-white grow flex items-center '}>
          <div className={'flex flex-row items-center'}>
            <IconBadge>
              <Clock size="48" />
            </IconBadge>
            <div className={'flex flex-col gap-1'}>
              <div className={'flex text-[#A2A2A2] text-[8px]'}>
                <ButtonBadge value={'Uptime'} />
              </div>
              <div className={'flex flex-row items-center gap-3'}>
                <div>99.944%</div>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

export default GainsModule
