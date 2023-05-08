const Container = ({ children }: { children: React.ReactNode }) => {
    return (
        <div className={'border bg-blue-200 rounded-xl p-4 gap-3 flex flex-col'}>
            {children}
        </div>
    )
}

export default Container