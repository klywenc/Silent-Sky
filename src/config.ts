export type LaneKind = "white" | "black";

export interface Lane {
  id: string;
  label: string;
  kind: LaneKind;
  row: "top" | "bottom";
  col: number;
}

export interface ControllerLayout {
  name: string;
  scratchSide: "left" | "right";
  lanes: Lane[];
}

export const IIDX_SP: ControllerLayout = {
  name: "IIDX SP",
  scratchSide: "left",
  lanes: [
    { id: "k1", label: "1", kind: "white", row: "bottom", col: 1 },
    { id: "k2", label: "2", kind: "black", row: "top", col: 2 },
    { id: "k3", label: "3", kind: "white", row: "bottom", col: 3 },
    { id: "k4", label: "4", kind: "black", row: "top", col: 4 },
    { id: "k5", label: "5", kind: "white", row: "bottom", col: 5 },
    { id: "k6", label: "6", kind: "black", row: "top", col: 6 },
    { id: "k7", label: "7", kind: "white", row: "bottom", col: 7 },
  ],
};

export const SCRATCH_ID = "scratch";

export const MOCK_KEYMAP: Record<string, string> = {
  KeyS: "k1",
  KeyD: "k2",
  KeyF: "k3",
  Space: "k4",
  KeyJ: "k5",
  KeyK: "k6",
  KeyL: "k7",
};

export const MOCK_SCRATCH: Record<string, "up" | "down"> = {
  ArrowUp: "up",
  ArrowDown: "down",
};
