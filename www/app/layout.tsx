import type { Metadata } from "next"
import type React from "react"
import { Crimson_Text, Geist_Mono, Nunito_Sans } from "next/font/google"

import { Analytics } from "@vercel/analytics/next"

import { SiteFooter } from "@/components/site-shell"
import { SiteHeader } from "@/components/site-header"

import "./globals.css"

const crimsonText = Crimson_Text({
  subsets: ["latin"],
  variable: "--font-crimson-text",
  weight: ["400", "600", "700"],
})
const geistMono = Geist_Mono({ subsets: ["latin"], variable: "--font-geist-mono" })
const nunitoSans = Nunito_Sans({ subsets: ["latin"] })

export const metadata: Metadata = {
  title: "Tea Docs",
  description:
    "Source-backed documentation for Tea: a strongly typed scripting language that compiles to native code.",
}

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode
}>) {
  return (
    <html lang="en">
      <body
        className={`${crimsonText.variable} ${geistMono.variable} ${nunitoSans.className} min-h-screen bg-background text-foreground antialiased`}
      >
        <div className="flex min-h-screen flex-col">
          <SiteHeader />
          <main className="flex-1">{children}</main>
          <SiteFooter />
        </div>
        <Analytics />
      </body>
    </html>
  )
}
