import { useState, useEffect } from 'react'

const ReputationMeter = ({
  outer,
  middle,
  inner,
}: {
  outer: number
  middle: number
  inner: number
}) => {
  const [parentThic, setParentThic] = useState(12)
  const [thickness, setThickness] = useState(18)

  useEffect(() => {
    setThickness(parentThic * 1.5)
  }, [parentThic])

  const OuterRing = ({ value }) => (
    <>
      <div
        className="radial-progress absolute text-grey-400"
        style={{
          '--value': 100,
          '--size': `${parentThic}rem`,
          '--thickness': `${thickness}px`,
        }}
      />
      <div
        className="radial-progress left-0 absolute text-earth"
        style={{
          '--value': value,
          '--size': `${parentThic}rem`,
          '--thickness': `${thickness}px`,
        }}
      />
    </>
  )

  const MiddleRing = ({ value }) => (
    <>
      <div
        className="radial-progress absolute left-[12.5%] top-[12.5%] text-grey-400"
        style={{
          '--value': 100,
          '--size': `${parentThic * 0.75}rem`,
          '--thickness': `${thickness}px`,
        }}
      />
      <div
        className="radial-progress text-venus absolute left-[12.5%] top-[12.5%]"
        style={{
          '--value': value,
          '--size': `${parentThic * 0.75}rem`,
          '--thickness': `${thickness}px`,
        }}
      />
    </>
  )

  const InnerRing = ({ value }) => (
    <>
      <div
        className="radial-progress text-grey-400 absolute left-[25%] top-[25%]"
        style={{
          '--value': 100,
          '--size': `${parentThic * 0.5}rem`,
          '--thickness': `${thickness}px`,
        }}
      />

      <div
        className="radial-progress text-mars absolute left-[25%] top-[25%]"
        style={{
          '--value': value,
          '--size': `${parentThic * 0.5}rem`,
          '--thickness': `${thickness}px`,
        }}
      />
    </>
  )

  return (
    <div className={`relative w-[193px] h-[193px] mx-auto`}>
      <OuterRing value={outer} />
      <MiddleRing value={middle} />
      <InnerRing value={inner} />
      <img
        alt={
          "gradient because raidal progress component doesn't support gradients"
        }
        src={'/gradient.png'}
        className={'w-[193px] h-[193px] absolute mix-blend-overlay'}
      />
    </div>
  )
}

export default ReputationMeter
