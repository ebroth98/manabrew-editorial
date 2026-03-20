import type { HandDisplayProps } from "../game.types";
import { HandDisplayNormal } from "./HandDisplayNormal";
import { HandDisplayCool } from "./HandDisplayCool";
import { usePreferencesStore } from "@/stores/usePreferencesStore";

export function HandDisplay(props: HandDisplayProps) {
  const handDisplayMode = usePreferencesStore((s) => s.handDisplayMode);

  if (handDisplayMode === "normal") {
    return <HandDisplayNormal {...props} />;
  }
  return <HandDisplayCool {...props} />;
}
