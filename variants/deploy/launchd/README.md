`mkdir -p "$HOME/Library/Application Support/TEZAUR" && cp org.tezaur.serve.plist ~/Library/LaunchAgents/ && launchctl load ~/Library/LaunchAgents/org.tezaur.serve.plist`
launchd does not expand `~` in environment values: edit the path to your real home folder first.
