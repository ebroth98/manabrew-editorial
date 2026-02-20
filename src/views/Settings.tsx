import { usePreferencesStore } from "@/stores/usePreferencesStore";
import { Label } from "@/components/ui/label";

const FLASH_MIN = 200;
const FLASH_MAX = 2000;
const FLASH_STEP = 100;

export default function Settings() {
  const { flashDurationMs, setFlashDurationMs } = usePreferencesStore();

  return (
    <div className="max-w-xl mx-auto py-8 px-4 space-y-8">
      <h1 className="text-2xl font-bold">Preferences</h1>

      <section className="space-y-4">
        <h2 className="text-lg font-semibold">Game Animations</h2>

        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <Label htmlFor="flash-duration">Flash duration</Label>
            <span className="text-sm font-mono text-muted-foreground">
              {flashDurationMs}ms
            </span>
          </div>
          <input
            id="flash-duration"
            type="range"
            min={FLASH_MIN}
            max={FLASH_MAX}
            step={FLASH_STEP}
            value={flashDurationMs}
            onChange={(e) => setFlashDurationMs(Number(e.target.value))}
            className="w-full accent-primary"
          />
          <p className="text-xs text-muted-foreground">
            How long card-play and turn-start flashes stay on screen.
          </p>
        </div>
      </section>
    </div>
  );
}
