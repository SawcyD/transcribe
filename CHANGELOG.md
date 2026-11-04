# Changelog

Each `## vX.Y.Z` section is published as that version's GitHub release notes
and shown in VoiceFlow's "Update available" dialog, so write it for users:
what changed for them, not how it was built.

## v1.2.0

**Talk to your assistant, out loud.** Press **Ctrl+Alt+V** for a spoken back-and-forth conversation. A purple dock shows the assistant talking above your own, and conversations are saved so it can remember them later.

### New
- **Ask about your screen** (Ctrl+Alt+B, formerly "Command Mode"): ask out loud and the answer pops up in a bubble right above the dock. It looks at the window you were working in and can explain errors, code, or anything you're stuck on. You can follow up, copy, or have it read aloud.
- **Saved chats and memory:** assistant conversations are kept on this PC, and you can ask things like "remember that interview I mentioned?"
- **Natural voices:** answers are read aloud with Deepgram voices. Pick one and preview it under Assistant.
- **Live captions:** see your words appear in the dock as you speak.
- **Today on Home:** words dictated, time saved compared to typing, and your daily streak.
- **History:** star the dictations you want to keep forever (S), search with Ctrl+F, copy with Ctrl+C, export as Markdown, and add a misheard word to your dictionary in two clicks.
- **"Can't hear you":** the dock tells you when your microphone isn't picking anything up.
- **Choose where VoiceFlow's sounds play:** pick an output device in Settings › Audio (including SteelSeries Sonar channels like Media or Chat). The voice agent, read-aloud answers and tones all follow it, and a test button checks it.
- **Tabs in Settings and Dictation,** so you don't have to scroll past everything to find one option.
- **A proper loading screen,** with a little animation, and animations throughout the app.
- **Settings save automatically.** No more Save button at the bottom of the page.

### Faster
- Recording starts sooner: VoiceFlow keeps a connection ready between dictations (you can turn this off in Dictation settings).
- Short phrases skip the AI cleanup step and are typed right away.
- Audio is sent in a lighter format, which helps on slow connections.
- The app opens faster: each window now loads only what it needs.

### Fixed
- The dock no longer says "Could not paste automatically" when the text was actually pasted, and it now explains what really went wrong.
- After an error, your next shortcut press works straight away instead of being ignored.
- Holding Ctrl+Alt+B no longer triggers the same shortcut in the app you're using, and Ctrl+Win+Space no longer switches your keyboard layout.
- The assistant now looks at your app, not VoiceFlow's own window.
- The first words of a dictation are no longer cut off on slow connections.
- Slow apps (Slack, Discord, VS Code) no longer occasionally paste your previous clipboard.
- Words like "bulletin" are no longer turned into bullet points.
- Windows shortcuts that start with Ctrl+Win (like Ctrl+Win+→ to switch desktops) and quick accidental taps no longer start a dictation or show "Didn't catch that".
- The dock's outline is no longer cut off, and the docks stay centred on high-DPI (125–150%) displays.
- Startup no longer flashes black and then white.

### Looks
- A cleaner, rounder design across the app, a sidebar that stays out of the way, and a History page that's easy to read.
