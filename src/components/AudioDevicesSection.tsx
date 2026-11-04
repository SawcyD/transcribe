import { Volume2 } from "lucide-react";
import { useEffect, useState } from "react";
import { ComboBox } from "@memora/ui";
import { Button } from "./common/Button";
import { SettingsRow, SettingsSection } from "./fluent/SettingsRow";
import { events, native } from "../lib/native";
import type { AppSettings } from "../types/models";

/**
 * Microphone and output device pickers. VoiceFlow's own sounds (voice agent,
 * read-aloud answers, tones) play on the chosen output, so they can be routed
 * to a mixer channel such as SteelSeries Sonar's Media or Chat device.
 */
export function AudioDevicesSection({ settings, update }: { settings: AppSettings; update: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => void }) {
  const [microphones, setMicrophones] = useState<string[]>([]);
  const [outputs, setOutputs] = useState<string[]>([]);
  const [testing, setTesting] = useState(false);
  const [level, setLevel] = useState(0);
  const [played, setPlayed] = useState(false);

  const refresh = () => {
    void native.microphones().then(setMicrophones);
    void native.outputDevices().then(setOutputs);
  };
  useEffect(refresh, []);

  useEffect(() => {
    if (!testing) return;
    const pending = events.audio((payload) => setLevel(payload.rms));
    return () => void pending.then((unlisten) => unlisten());
  }, [testing]);

  const missing = (name: string | null, list: string[]) => Boolean(name) && list.length > 0 && !list.includes(name ?? "");

  return (
    <SettingsSection title="Audio devices" description="Pick where VoiceFlow listens and where its own sounds play. Devices plugged in later appear after Refresh.">
      <SettingsRow
        label="Microphone"
        description={missing(settings.microphoneName, microphones) ? "This microphone isn't connected right now; the Windows default is used until it is." : "Used for dictation, Ask about your screen, and the voice agent."}
        action={<ComboBox label="Microphone" value={settings.microphoneName ?? ""} onChange={(value) => update("microphoneName", value || null)} options={[{ value: "", label: "Windows default" }, ...microphones.map((name) => ({ value: name, label: name }))]} />}
      />
      <SettingsRow
        label="Test microphone"
        description="Starts a short hands-free session so you can watch the level, then stops it."
        action={
          <span className="level-control">
            <span className="level-meter" role="meter" aria-label="Input level" aria-valuenow={Math.round(level * 100)} aria-valuemin={0} aria-valuemax={100}>
              <i style={{ width: `${Math.min(100, Math.round(level * 140))}%` }} />
            </span>
            <Button
              variant="secondary"
              onClick={async () => {
                if (testing) {
                  await native.finish();
                  setTesting(false);
                  setLevel(0);
                } else {
                  await native.start("hands_free");
                  setTesting(true);
                }
              }}
            >
              {testing ? "Stop test" : "Test"}
            </Button>
          </span>
        }
      />
      <SettingsRow
        label="Output device"
        description={
          missing(settings.outputDeviceName, outputs)
            ? "This device isn't available right now; the Windows default is used until it is."
            : "Where the voice agent, read-aloud answers and tones play. Pick a Sonar channel (e.g. Media or Chat) to route them through your mixer."
        }
        action={<ComboBox label="Output device" value={settings.outputDeviceName ?? ""} onChange={(value) => update("outputDeviceName", value || null)} options={[{ value: "", label: "Windows default" }, ...outputs.map((name) => ({ value: name, label: name }))]} />}
      />
      <SettingsRow
        label="Test output"
        description={played ? "Did you hear a two-note chime? If not, try another device." : "Plays a short chime on the device above."}
        action={
          <span className="level-control">
            <Button
              variant="secondary"
              icon={<Volume2 size={15} />}
              onClick={() => {
                setPlayed(true);
                void native.testOutputDevice(settings.outputDeviceName);
              }}
            >
              Play test sound
            </Button>
            <Button variant="ghost" onClick={refresh}>
              Refresh
            </Button>
          </span>
        }
      />
    </SettingsSection>
  );
}
