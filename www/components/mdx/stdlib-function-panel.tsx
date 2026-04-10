import { FunctionPanel } from "@/components/mdx/function-card"
import { parseStdlibFunctions } from "@/lib/repo"

interface StdlibFunctionPanelProps {
  sourcePath: string
}

export function StdlibFunctionPanel({ sourcePath }: StdlibFunctionPanelProps) {
  const functions = parseStdlibFunctions(sourcePath)
  return <FunctionPanel functions={functions} />
}
