/** Five-token packs. ice paper · ash chrome · record mid/accent · slate muted · void field. */

export type PackId = string;

export type Pack = {
  id: PackId;
  name: string;
  ice: string;
  ash: string;
  record: string;
  slate: string;
  void: string;
  hot: "void" | "record";
  /** Transcript ink. Defaults to ash. Use when ash is a saturated accent. */
  lcdInk?: string;
};

export const PACKS: Pack[] = [
  {
    id: "petri-grey",
    name: "Petri Grey",
    ice: "#f3f0ea",
    ash: "#b7b7b7",
    record: "#8c8c8c",
    slate: "#525252",
    void: "#000000",
    hot: "void",
  },
  {
    id: "field-signal",
    name: "Field Signal",
    ice: "#c0cdce",
    ash: "#dab71f",
    record: "#964444",
    slate: "#789044",
    void: "#3c4c85",
    hot: "record",
    lcdInk: "#c0cdce",
  },
  {
    id: "midnight-shadows",
    name: "Midnight Shadows",
    ice: "#f7f4ee",
    ash: "#9aa4b2",
    record: "#c41e3a",
    slate: "#1b2430",
    void: "#0a0a0f",
    hot: "record",
  },
  {
    id: "obsidian-depths",
    name: "Obsidian Depths",
    ice: "#e8f4f4",
    ash: "#6aa8a8",
    record: "#1a6b73",
    slate: "#12343c",
    void: "#071218",
    hot: "record",
  },
  {
    id: "twilight-hues",
    name: "Twilight Hues",
    ice: "#f5f5f5",
    ash: "#da70d6",
    record: "#8a2be2",
    slate: "#4b0082",
    void: "#160024",
    hot: "record",
  },
  {
    id: "ravens-wing",
    name: "Raven’s Wing",
    ice: "#f2f2f2",
    ash: "#bfbfbf",
    record: "#7a7a7a",
    slate: "#2a2a2a",
    void: "#000000",
    hot: "void",
  },
  {
    id: "stormy-night",
    name: "Stormy Night",
    ice: "#e8f7ff",
    ash: "#00bfff",
    record: "#d63031",
    slate: "#2f3640",
    void: "#12151a",
    hot: "record",
  },
  {
    id: "charcoal-dreams",
    name: "Charcoal Dreams",
    ice: "#f0f0f0",
    ash: "#b2b2b2",
    record: "#7a7a7a",
    slate: "#3b3b3b",
    void: "#161616",
    hot: "void",
  },
  {
    id: "deep-sea-abyss",
    name: "Deep Sea Abyss",
    ice: "#aeeeee",
    ash: "#5ec8c8",
    record: "#1a6b8a",
    slate: "#003056",
    void: "#001f3f",
    hot: "record",
  },
  {
    id: "forest-dusk",
    name: "Forest Dusk",
    ice: "#f0f3f4",
    ash: "#a9dfbf",
    record: "#6b8e23",
    slate: "#4a5e3d",
    void: "#2e3a24",
    hot: "record",
  },
  {
    id: "gothic-elegance",
    name: "Gothic Elegance",
    ice: "#e8d8e0",
    ash: "#a45dba",
    record: "#7a3d8c",
    slate: "#3d2a44",
    void: "#1a1018",
    hot: "record",
  },
  {
    id: "sable-night",
    name: "Sable Night",
    ice: "#e8e8e8",
    ash: "#a0a0a0",
    record: "#6a6a6a",
    slate: "#3a3a3a",
    void: "#1c1c1c",
    hot: "void",
  },
  {
    id: "dark-forest",
    name: "Dark Forest",
    ice: "#f0f0f0",
    ash: "#a4c8e1",
    record: "#4a7fa3",
    slate: "#2c3e50",
    void: "#1b3a57",
    hot: "record",
  },
  {
    id: "shadowed-earth",
    name: "Shadowed Earth",
    ice: "#ededed",
    ash: "#bfa6a6",
    record: "#8b5b5b",
    slate: "#6b4f4f",
    void: "#4b3d3d",
    hot: "record",
  },
  {
    id: "nightfall-bliss",
    name: "Nightfall Bliss",
    ice: "#d3d3d3",
    ash: "#a8a8a8",
    record: "#7a7b7a",
    slate: "#4b4a48",
    void: "#2c2a29",
    hot: "void",
  },
  {
    id: "dusks-embrace",
    name: "Dusk’s Embrace",
    ice: "#efe6f6",
    ash: "#a77bca",
    record: "#7a5b9d",
    slate: "#5b3f8d",
    void: "#3e2a47",
    hot: "record",
  },
  {
    id: "enigmatic-charcoal",
    name: "Enigmatic Charcoal",
    ice: "#e8e8e8",
    ash: "#afafaf",
    record: "#7a7a7a",
    slate: "#3a3a3a",
    void: "#1f1f1f",
    hot: "void",
  },
];

export const DEFAULT_PACK_ID = "field-signal";

export function packById(id: string | null | undefined): Pack {
  return PACKS.find((p) => p.id === id) || PACKS[0];
}

export function hexRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "").trim();
  const n = h.length === 3 ? h.split("").map((c) => c + c).join("") : h;
  const v = parseInt(n.slice(0, 6), 16);
  if (Number.isNaN(v)) return [0, 0, 0];
  return [((v >> 16) & 255) / 255, ((v >> 8) & 255) / 255, (v & 255) / 255];
}
