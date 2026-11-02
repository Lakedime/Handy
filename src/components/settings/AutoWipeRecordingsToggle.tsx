import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface AutoWipeRecordingsToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/** Hardened build: see `auto_wipe_recordings` in src-tauri/src/settings.rs. */
export const AutoWipeRecordingsToggle: React.FC<
  AutoWipeRecordingsToggleProps
> = ({ descriptionMode = "tooltip", grouped = false }) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("auto_wipe_recordings") ?? true;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(value) => updateSetting("auto_wipe_recordings", value)}
      isUpdating={isUpdating("auto_wipe_recordings")}
      label={t("settings.advanced.autoWipeRecordings.label")}
      description={t("settings.advanced.autoWipeRecordings.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    />
  );
};
