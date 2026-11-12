# Changelog

Each `## vX.Y.Z` section is published as that version's GitHub release notes
and shown in VoiceFlow's "Update available" dialog, so write it for users:
what changed for them, not how it was built.

## v2.0.0

**Meet Flow.** VoiceFlow's voice assistant now has a name, a face and a lot more it can do. Say **"Hey Flow"** (or press **Ctrl+Alt+V**) and a glowing orb rises from the taskbar to talk with you.

### New
- **The orb:** it swells with whoever is talking, shifts colour with the mood of the conversation, and sits wherever you drag it.
- **"Hey Flow":** start a conversation hands-free. Listening for the phrase happens on your PC, on the microphone you chose in Settings. Turn it on under Assistant › Flow voice agent.
- **Personalities:** Friendly, Smug, Professional, Hype, Blunt or Mentor. Same help, different attitude.
- **36 new voices** and an expressivity setting, from calm to animated.
- **flow.md:** tell Flow about yourself once, and say "Flow, remember…" to add to it.
- **Whiteboard:** a shared board you and Flow can both draw on. Flow sketches flowcharts and notes a piece at a time while it explains, reads what you drew, can add pictures from the web, and boards are saved by name.
- **Planner:** a glass day planner with a timeline and the order your tasks depend on. Ask "what's my plan for today?", "add study for my exam to tomorrow morning" or "push my 3 PM task to tomorrow".
- **Type or paste to Flow** during a conversation, including screenshots and images.
- **Web search:** with a free Tavily key, Flow looks things up and can put what it finds on the whiteboard with links.
- **Mock interviews:** ask Flow to interview you, behavioural or technical. It can look at your code when you're done.
- **Flow can see your screen** when you ask, and **open apps and websites** ("open OBS"). It can't close, change or delete anything.
- **Numbers as digits** in dictation ("nine hundred" becomes "900").
- **Say goodbye to hang up:** Flow ends the call when you're done.

### Better
- Flow ignores claps, snaps and other short sounds.
- The Assistant page is split into Chat, Flow voice agent and Chat settings.
- Smoother animations across the dock.

### Fixed
- Dictation failing with "Transcription failed" after the voice-activity update.
- Dialogs losing focus while you typed in them.
- Voice conversations saved one line at a time in your chats.

### Removed
- Buddy, the desktop companion. Flow takes its place.

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
