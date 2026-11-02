import React from "react";
import { useTranslation } from "react-i18next";
import { ToggleSwitch } from "../ui/ToggleSwitch";
import { useSettings } from "../../hooks/useSettings";

interface OfflineLockToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

/** Hardened build: see `offline_lock` in src-tauri/src/settings.rs. */
export const OfflineLockToggle: React.FC<OfflineLockToggleProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("offline_lock") ?? false;

  return (
    <ToggleSwitch
      checked={enabled}
      onChange={(value) => updateSetting("offline_lock", value)}
      isUpdating={isUpdating("offline_lock")}
      label={t("settings.advanced.offlineLock.label")}
      description={t("settings.advanced.offlineLock.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    />
  );
};
