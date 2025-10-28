# LSP Setup

The Tea language server (`tea-lsp`) provides IDE features such as hover docs, go-to definition, and diagnostics on top of the compiler pipeline. This guide explains how to build the binary, put it on your `PATH`, wire it into Neovim and VS Code, and debug common problems.

## Prerequisites

- **Rust toolchain** (1.75 or newer) – required to compile the server. Install via [rustup](https://rustup.rs/) if you have not already.
- **Tea source tree** – clone [github.com/bentruyman/tea-lang](https://github.com/bentruyman/tea-lang) and run commands from the repository root.
- Optional for VS Code: **Node.js 18+** and **npm** for managing a lightweight client extension.

## Building and Installing `tea-lsp`

From the repository root, compile the language server and place it in `~/.cargo/bin`:

```bash
cargo install --path tea-lsp --locked --force
```

The `--force` flag ensures upgrades overwrite any previous build. If you prefer to keep binaries inside the repo, you can also run:

```bash
cargo build -p tea-lsp --release
cp target/release/tea-lsp ./bin/
```

Make sure the directory containing the binary appears in your `PATH`. For example, append the following to your shell profile if you rely on `cargo install`:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Verifying the Installation

Run the server directly from a terminal to ensure it starts without crashing:

```bash
tea-lsp
```

The process stays attached to STDIN/STDOUT waiting for an LSP client. You can interrupt it with `Ctrl+C`. When debugging unexpected behaviour, start it with structured logging enabled:

```bash
RUST_LOG=tea_lsp=debug tea-lsp
```

## Neovim Integration

The easiest way to connect Neovim is through [`nvim-lspconfig`](https://github.com/neovim/nvim-lspconfig). Add the following Lua snippet to your Neovim configuration (for example `~/.config/nvim/lua/lsp/tea.lua`) and source it from `init.lua`.

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.tea_lsp then
  configs.tea_lsp = {
    default_config = {
      name = 'tea-lsp',
      cmd = { 'tea-lsp' },
      filetypes = { 'tea' },
      root_dir = lspconfig.util.root_pattern('Tea.toml', '.git'),
      single_file_support = true,
    },
  }
end

lspconfig.tea_lsp.setup({
  -- Set this if tea-lsp lives outside your PATH.
  -- cmd = { '/absolute/path/to/tea-lsp' },
})

-- Optional: make sure Neovim recognises *.tea files.
vim.filetype.add({ extension = { tea = 'tea' } })
```

After reloading Neovim, open a `.tea` file and run `:LspInfo` to confirm the server is attached. If you use a plugin manager (Lazy.nvim, Packer, etc.), drop the snippet into the appropriate setup callback. For remote workspaces, pass an absolute path in the `cmd` array so the client launches the correct binary.

## VS Code Integration

VS Code does not ship with a generic “register any LSP binary” workflow, so create a lightweight client extension that spawns `tea-lsp` over stdio.

1. Ensure Node.js and npm are available.
2. Create a folder for the extension, e.g. `~/dev/tea-lsp-vscode`, and initialise it:

   ```bash
   mkdir -p ~/dev/tea-lsp-vscode
   cd ~/dev/tea-lsp-vscode
   npm init -y
   npm install vscode-languageclient --save
   ```

3. Replace the generated `package.json` with the following minimal manifest (feel free to update the `publisher` or `version` fields to suit your environment):

   ```json
   {
     "name": "tea-language-support",
     "displayName": "Tea Language Support",
     "description": "Connects VS Code to the tea-lsp binary",
     "version": "0.0.1",
     "publisher": "local",
     "engines": { "vscode": "^1.84.0" },
     "activationEvents": ["onLanguage:tea"],
     "contributes": {
       "languages": [
         {
           "id": "tea",
           "aliases": ["Tea", "tea"],
           "extensions": [".tea"]
         }
       ],
       "configuration": {
         "type": "object",
         "title": "Tea",
         "properties": {
           "tea.lspPath": {
             "type": "string",
             "default": "",
             "description": "Optional absolute path to the tea-lsp binary. Leave blank to use PATH."
           }
         }
       },
       "commands": [
         {
           "command": "tea-lsp.restart",
           "title": "Tea LSP: Restart Server"
         }
       ]
     },
     "main": "./extension.js",
     "dependencies": {
       "vscode-languageclient": "^9.0.0"
     }
   }
   ```

4. Create `extension.js` beside the manifest with the client bootstrap logic:

   ```javascript
   const vscode = require('vscode')
   const { LanguageClient } = require('vscode-languageclient/node')

   let client

   function activate(context) {
     const config = vscode.workspace.getConfiguration('tea')
     const command = config.get('lspPath') || process.env.TEA_LSP_PATH || 'tea-lsp'

     const serverOptions = {
       command,
       args: [],
       options: { env: process.env },
     }

     const clientOptions = {
       documentSelector: [{ scheme: 'file', language: 'tea' }],
       synchronize: {
         fileEvents: vscode.workspace.createFileSystemWatcher('**/*.tea'),
       },
     }

     client = new LanguageClient('tea-lsp', 'Tea Language Server', serverOptions, clientOptions)
     context.subscriptions.push(client.start())

     context.subscriptions.push(
       vscode.commands.registerCommand('tea-lsp.restart', async () => {
         if (!client) {
           return
         }
         await client.stop()
         await client.start()
         vscode.window.showInformationMessage('Tea LSP restarted')
       })
     )
   }

   function deactivate() {
     if (!client) {
       return undefined
     }
     return client.stop()
   }

   module.exports = { activate, deactivate }
   ```

5. Package and install the extension:

   ```bash
   npm install -g @vscode/vsce       # if you do not already have vsce
   npx vsce package                  # creates tea-language-support-0.0.1.vsix
   code --install-extension tea-language-support-0.0.1.vsix
   ```

6. Restart VS Code. Open a Tea file and check **View → Output → Tea Language Server** for startup logs. Set the optional `"tea.lspPath"` setting (User or Workspace scope) if VS Code cannot find `tea-lsp` on your `PATH`.

## Troubleshooting

- **Server binary not found** – confirm `tea-lsp` is in your `PATH` (`which tea-lsp`). Configure `cmd` in Neovim or set `tea.lspPath` in VS Code if the binary lives elsewhere.
- **Neovim reports “command not executable”** – ensure the binary has the executable bit (`chmod +x` if you copied it manually) and that the absolute path has no spaces. Use `:checkhealth` to inspect remote shell PATH differences.
- **No diagnostics or hover info** – make sure the workspace you opened contains Tea sources that compile. Run `RUST_LOG=tea_lsp=debug tea-lsp` in a terminal to inspect incoming requests. On VS Code, open the *Tea Language Server* output channel for verbose logs.
- **Crashes on startup** – delete the cached binary and rebuild with `cargo install --force`. Out-of-date dependencies occasionally cause handshake mismatches.
- **Neovim attaches but completions are empty** – verify your configuration registers the `tea` filetype (`:set filetype?`). If you use Tree-sitter, ensure a grammar exists or disable it temporarily while testing.
- **VS Code does not recognise `.tea` files** – confirm the extension installed successfully via `code --list-extensions | grep tea-language-support`. You can also add a file association in settings: `"files.associations": { "*.tea": "tea" }`.

Happy hacking! With `tea-lsp` in place you should now have hover docs, go-to definition, and diagnostics wherever you edit Tea sources.
