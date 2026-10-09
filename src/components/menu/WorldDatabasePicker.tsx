import { useId } from "react";
import { useTranslation } from "react-i18next";

/** One entry from the backend's `list_world_databases` command. */
export interface WorldDatabaseInfo {
  id: string;
  name: string;
  description: string;
  team_count: number;
  player_count: number;
  history_mode: string;
  base_year: number | null;
  source: string;
}

/** The id `list_world_databases` gives the built-in generated world. */
export const GENERATED_WORLD_ID = "random";

interface WorldDatabasePickerProps {
  databases: WorldDatabaseInfo[];
  selectedId: string;
  onSelect: (id: string) => void;
}

/**
 * Lets the player start from a world file in the `databases` folder, such as a CM 01/02 world
 * written by `cm0102-import`, instead of the generated world. Shows nothing when only the
 * generated world exists.
 */
export default function WorldDatabasePicker({
  databases,
  selectedId,
  onSelect,
}: WorldDatabasePickerProps) {
  const { t } = useTranslation();
  const groupName = useId();
  const files = databases.filter((db) => db.id !== GENERATED_WORLD_ID);
  if (files.length === 0) return null;

  const options = [
    { id: GENERATED_WORLD_ID, label: t("menu.generatedWorld"), detail: "" },
    ...files.map((db) => ({
      id: db.id,
      label: db.name,
      detail: t("menu.worldDatabaseDetail", { teams: db.team_count, players: db.player_count }),
    })),
  ];

  return (
    <fieldset className="mb-4 flex flex-col gap-2">
      <legend className="mb-1 text-sm font-semibold text-gray-700 dark:text-gray-200">
        {t("menu.worldDatabase")}
      </legend>
      {options.map((option) => (
        <label
          key={option.id}
          className="flex cursor-pointer items-start gap-2 rounded-lg border border-gray-200 p-2 text-sm dark:border-navy-600"
        >
          <input
            type="radio"
            name={groupName}
            value={option.id}
            checked={selectedId === option.id}
            onChange={() => onSelect(option.id)}
            className="mt-1"
          />
          <span>
            <span className="block font-medium text-gray-900 dark:text-white">{option.label}</span>
            {option.detail && (
              <span className="block text-xs text-gray-500 dark:text-gray-400">
                {option.detail}
              </span>
            )}
          </span>
        </label>
      ))}
      {selectedId !== GENERATED_WORLD_ID && (
        <p className="text-xs text-gray-500 dark:text-gray-400">{t("menu.worldDatabaseHint")}</p>
      )}
    </fieldset>
  );
}
