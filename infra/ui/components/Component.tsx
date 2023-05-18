const Container = ({ children }: { children: React.ReactNode }) => {
  return (
    <div className={'border rounded-xl p-4 gap-3 flex flex-col'}>
      {children}
    </div>
  )
}

export default Container
