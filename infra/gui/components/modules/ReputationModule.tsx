import RepRowItem from '@/components/RepRowItem'
import ReputationMeter from '@/components/ReputationMeter'

const ReputationModule = () => {
  return (
    <div className="border rounded-3xl p-6">
      <div className="2xl:flex">
        <div className="grow" />
        <div className="justify-center m-4">
          <ReputationMeter outer={70} middle={75} inner={50} />
        </div>
        <div className="grow" />
        <div className="justify-around flex items-start flex-col gap-3">
          <RepRowItem
            title={'Reputation Score'}
            btnText={'950'}
            pctChange={-4.7}
            displayColor={'bg-neon-energy'}
          />
          <RepRowItem
            title={'Average Reputation'}
            btnText={'500'}
            pctChange={4.8}
            displayColor={'bg-neon-bonding'}
          />
          <RepRowItem
            title={'Current DTS'}
            btnText={'60%'}
            pctChange={2.3}
            displayColor={'bg-neon-solar'}
          />
        </div>
      </div>
    </div>
  )
}

export default ReputationModule
