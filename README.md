# browser-mcp

## Chrome DevTools for agents

This is a port of [chrome-devtools-mcp](https://github.com/ChromeDevTools/chrome-devtools-mcp) to Rust and gives your agent the ability to drive Chrome and Chromium based browsers.

I have plans to implement the W3C web driver spec to enable driving other browsers, but this is a good starting point for now.

## Zero Dependencies

This is distributed as a single binary with no runtime or external dependencies. Just download it, add it to your `$PATH`, register it in your harnesses, and never think about it again.

## Installation

Download the binary from [GitHub Releases](https://github.com/alshdavid/browser-mcp/releases/latest), put it under `$HOME/.local/bin` and add it to your `$PATH`

```bash
# Linux AMD64 (Change for your platform)
curl -L --url https://github.com/alshdavid/browser-mcp/releases/latest/download/browser-mcp-linux-amd64.tar.gz | tar -xvzf - -C $HOME/.local/bin

# Add to PATH if not already there:
echo "\nexport \PATH=\$PATH:\$HOME/.local/bin\n" >> $HOME/.zshrc
echo "\nexport \PATH=\$PATH:\$HOME/.local/bin\n" >> $HOME/.bashrc
```

## Updating

Once installed, run `browser-mcp update` and it will self-update.
