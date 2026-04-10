interface FunctionSignatureCardProps {
  signature: string
  description: string
}

export function FunctionSignatureCard({ signature, description }: FunctionSignatureCardProps) {
  const nameMatch = signature.match(/^(pub\s+)?def\s+(\w+)/)
  const fnName = nameMatch?.[2]

  let displaySignature
  if (fnName) {
    const idx = signature.indexOf(fnName)
    displaySignature = (
      <>
        {signature.slice(0, idx)}
        <span className="fn-name">{fnName}</span>
        {signature.slice(idx + fnName.length)}
      </>
    )
  } else {
    displaySignature = signature
  }

  return (
    <div className="py-4 first:pt-0 last:pb-0">
      <div className="fn-signature">{displaySignature}</div>
      <p className="mt-2 px-1 text-sm leading-6 text-muted-foreground">
        {description || "No inline docs found."}
      </p>
    </div>
  )
}

interface FunctionPanelProps {
  functions: { signature: string; description: string }[]
}

export function FunctionPanel({ functions }: FunctionPanelProps) {
  return (
    <div className="section-band surface-quiet texture-grid-fine overflow-hidden p-5 md:p-6">
      <div className="relative z-10">
        <p className="text-xs font-semibold uppercase tracking-[0.2em] text-primary">Exported functions</p>
        <div className="mt-4 divide-y divide-border/50">
          {functions.map((fn) => (
            <FunctionSignatureCard key={fn.signature} signature={fn.signature} description={fn.description} />
          ))}
        </div>
      </div>
    </div>
  )
}
