import { describe, expect, it, vi } from "vitest";
import { fireEvent, render, screen } from "@testing-library/react";
import WorldDatabasePicker, {
  GENERATED_WORLD_ID,
  type WorldDatabaseInfo,
} from "./WorldDatabasePicker";

const generated: WorldDatabaseInfo = {
  id: GENERATED_WORLD_ID,
  name: "be.world.random",
  description: "",
  team_count: 440,
  player_count: 9680,
  history_mode: "generated",
  base_year: null,
  source: "builtin",
};

const cm0102: WorldDatabaseInfo = {
  id: "file:/data/databases/cm0102-world.json",
  name: "Championship Manager 01/02",
  description: "",
  team_count: 360,
  player_count: 13022,
  history_mode: "reference",
  base_year: 2001,
  source: "user",
};

describe("WorldDatabasePicker", () => {
  it("renders nothing when only the generated world exists", () => {
    // Given only the built-in world, when the picker renders, then it shows no choice at all.
    const { container } = render(
      <WorldDatabasePicker
        databases={[generated]}
        selectedId={GENERATED_WORLD_ID}
        onSelect={vi.fn()}
      />,
    );

    expect(container).toBeEmptyDOMElement();
  });

  it("offers a world file and reports its id when chosen", () => {
    // Given a CM 01/02 world file, when the player picks it, then its file id is reported.
    const onSelect = vi.fn();
    render(
      <WorldDatabasePicker
        databases={[generated, cm0102]}
        selectedId={GENERATED_WORLD_ID}
        onSelect={onSelect}
      />,
    );

    fireEvent.click(screen.getByLabelText(/Championship Manager 01\/02/));

    expect(onSelect).toHaveBeenCalledWith("file:/data/databases/cm0102-world.json");
  });
});
