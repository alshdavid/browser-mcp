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

## Usage

```bash
# Launch a new Chrome instance (default)
browser-mcp stdio

# Launch headless
browser-mcp stdio --headless

# Use a specific Chrome binary
browser-mcp stdio --chrome-binary-path /usr/bin/chromium

# Attach to your running Chrome instead of launching one
browser-mcp stdio --auto-connect
```

`--auto-connect` (alias `--autoConnect`) attaches to the Chrome you already have
open, signing in with your existing profile. Start Chrome at least once and
enable the remote debugging server at `chrome://inspect/#remote-debugging`; the
port Chrome writes to its profile's `DevToolsActivePort` file is what the flag
connects to. Chrome is left running when the server exits.

## Lazy startup

Starting the server does not start Chrome. The browser is launched, or attached
to, on the first tool call that needs it, and that instance is then reused for
the life of the process. A session that only lists tools never opens a browser,
and the first tool call pays the startup cost once.

When attaching to a browser that already has tabs open, the first call that
needs a page adopts the existing tab rather than reporting that no page exists.
