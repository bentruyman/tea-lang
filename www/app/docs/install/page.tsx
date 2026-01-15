import { Card } from "@/components/ui/card"
import { Button } from "@/components/ui/button"
import Link from "next/link"
import { CheckCircle2, AlertCircle } from "lucide-react"

export default function InstallPage() {
  return (
    <div className="space-y-12">
      {/* Header */}
      <div className="space-y-4">
        <h1 className="text-4xl font-bold text-balance">Installation Guide</h1>
        <p className="text-xl text-muted-foreground text-pretty leading-relaxed">
          Complete instructions for installing Tea on Linux, macOS, and Windows.
        </p>
      </div>

      {/* System Requirements */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">System Requirements</h2>
        <Card className="p-6 bg-card border-border">
          <div className="grid md:grid-cols-2 gap-6">
            <div>
              <h3 className="font-semibold text-lg mb-3 text-accent">Required</h3>
              <ul className="space-y-2">
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <span className="text-sm">Git 2.0+</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <span className="text-sm">C++ compiler (GCC 9+ or Clang 10+)</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <span className="text-sm">LLVM 14+ (for native compilation)</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <span className="text-sm">Make or CMake</span>
                </li>
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-accent mt-0.5 shrink-0" />
                  <span className="text-sm">2GB RAM minimum</span>
                </li>
              </ul>
            </div>
            <div>
              <h3 className="font-semibold text-lg mb-3 text-accent">Optional</h3>
              <ul className="space-y-2">
                <li className="flex items-start gap-2">
                  <CheckCircle2 className="h-5 w-5 text-muted-foreground mt-0.5 shrink-0" />
                  <span className="text-sm">Python 3.8+ (for build scripts)</span>
                </li>
              </ul>
            </div>
          </div>
        </Card>
      </div>

      {/* Linux Installation */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">Linux</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="text-xl font-semibold mb-4">Ubuntu / Debian</h3>
          <div className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install dependencies:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  sudo apt update{"\n"}
                  sudo apt install git build-essential cmake llvm-14 llvm-14-dev
                </code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Clone and build Tea:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea{"\n"}
                  make setup{"\n"}
                  sudo make install
                </code>
              </pre>
            </div>
          </div>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="text-xl font-semibold mb-4">Fedora / RHEL</h3>
          <div className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install dependencies:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">sudo dnf install git gcc-c++ cmake llvm-devel</code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Clone and build Tea:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea{"\n"}
                  make setup{"\n"}
                  sudo make install
                </code>
              </pre>
            </div>
          </div>
        </Card>

        <Card className="p-6 bg-card border-border">
          <h3 className="text-xl font-semibold mb-4">Arch Linux</h3>
          <div className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install dependencies:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">sudo pacman -S git base-devel cmake llvm</code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Clone and build Tea:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea{"\n"}
                  make setup{"\n"}
                  sudo make install
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* macOS Installation */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">macOS</h2>

        <Card className="p-6 bg-card border-border">
          <div className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install Xcode Command Line Tools:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">xcode-select --install</code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install Homebrew (if not already installed):</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
                </code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install LLVM:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">brew install llvm</code>
              </pre>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Clone and build Tea:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea{"\n"}
                  make setup{"\n"}
                  sudo make install
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Windows Installation */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">Windows</h2>

        <Card className="p-6 bg-card border-border">
          <div className="flex items-start gap-3 mb-4 p-3 bg-muted/50 rounded-md">
            <AlertCircle className="h-5 w-5 text-accent shrink-0 mt-0.5" />
            <p className="text-sm text-muted-foreground">
              Windows support is currently experimental. We recommend using WSL2 (Windows Subsystem for Linux) for the
              best experience.
            </p>
          </div>

          <h3 className="text-lg font-semibold mb-3">Option 1: WSL2 (Recommended)</h3>
          <div className="space-y-4 mb-6">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install WSL2:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">wsl --install</code>
              </pre>
            </div>
            <p className="text-sm text-muted-foreground">
              After WSL2 is installed, follow the Linux installation instructions above.
            </p>
          </div>

          <h3 className="text-lg font-semibold mb-3">Option 2: Native Windows</h3>
          <div className="space-y-4">
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install Visual Studio 2022 with C++ tools</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Install Git for Windows</p>
            </div>
            <div>
              <p className="text-sm text-muted-foreground mb-2">Clone and build:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">
                  git clone https://github.com/special-tea/tea.git{"\n"}
                  cd tea{"\n"}
                  cmake -B build{"\n"}
                  cmake --build build --config Release{"\n"}
                  cmake --install build
                </code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Verify Installation */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">Verify Installation</h2>
        <Card className="p-6 bg-card border-border">
          <p className="text-muted-foreground mb-4">After installation, verify that Tea is working correctly:</p>
          <div className="space-y-4">
            <div>
              <p className="text-sm font-semibold mb-2">Check version:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">tea --version</code>
              </pre>
            </div>
            <div>
              <p className="text-sm font-semibold mb-2">View help:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">tea --help</code>
              </pre>
            </div>
            <div>
              <p className="text-sm font-semibold mb-2">Run a simple test:</p>
              <pre className="bg-muted p-4 rounded-md overflow-x-auto">
                <code className="font-mono text-sm text-foreground">echo 'print("Tea is working!")' | tea run -</code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Troubleshooting */}
      <div className="space-y-4">
        <h2 className="text-3xl font-bold">Troubleshooting</h2>

        <Card className="p-6 bg-card border-border">
          <h3 className="font-semibold text-lg mb-3">Common Issues</h3>
          <div className="space-y-4">
            <div>
              <h4 className="font-semibold text-accent mb-2">Command not found: tea</h4>
              <p className="text-sm text-muted-foreground mb-2">
                Make sure <code className="text-accent">/usr/local/bin</code> is in your PATH:
              </p>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-xs text-foreground">export PATH="/usr/local/bin:$PATH"</code>
              </pre>
            </div>

            <div>
              <h4 className="font-semibold text-accent mb-2">LLVM not found</h4>
              <p className="text-sm text-muted-foreground mb-2">
                LLVM 14+ is required for Tea. Make sure it's properly installed and available in your PATH.
              </p>
            </div>

            <div>
              <h4 className="font-semibold text-accent mb-2">Permission denied during installation</h4>
              <p className="text-sm text-muted-foreground">
                Use <code className="text-accent">sudo</code> for the install step, or install to a user directory:
              </p>
              <pre className="bg-muted p-3 rounded-md overflow-x-auto">
                <code className="font-mono text-xs text-foreground">make install PREFIX=$HOME/.local</code>
              </pre>
            </div>
          </div>
        </Card>
      </div>

      {/* Next Steps */}
      <Card className="p-6 bg-muted/30 border-border">
        <h3 className="text-lg font-semibold mb-3">Next Steps</h3>
        <p className="text-muted-foreground mb-4">Now that Tea is installed, you're ready to start coding!</p>
        <div className="flex flex-wrap gap-3">
          <Button className="bg-accent text-accent-foreground hover:bg-accent/90" asChild>
            <Link href="/docs/getting-started">Getting Started Guide</Link>
          </Button>
          <Button variant="outline" asChild>
            <Link href="/examples">Browse Examples</Link>
          </Button>
          <Button variant="outline" asChild>
            <Link href="/playground">Try the Playground</Link>
          </Button>
        </div>
      </Card>
    </div>
  )
}
