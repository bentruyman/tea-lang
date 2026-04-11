import type { Metadata } from "next";

import { PlaygroundClient } from "./playground-client";

export const metadata: Metadata = {
  title: "Tea Playground",
  description: "Run browser-safe Tea code in a WASM-powered playground.",
};

export default function PlaygroundPage() {
  return <PlaygroundClient />;
}
