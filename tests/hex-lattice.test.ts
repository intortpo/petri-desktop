import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  HEX_SDF_GLSL,
  SQRT3,
  hexCenterCart,
  hexEdgeDistance,
  sdHexagon,
} from "../src/lib/hex-lattice.ts";

const size = 1;
const inradius = (SQRT3 / 2) * size;

test("hex SDF is near zero on a flat edge and larger at the cell center", () => {
  const center = hexEdgeDistance(0, 0, size);
  const edge = Math.abs(sdHexagon(0, inradius, inradius));
  const latticeEdge = hexEdgeDistance(0, inradius, size);
  assert.ok(edge < 0.02, `flat-edge sdHexagon was ${edge}`);
  assert.ok(latticeEdge < 0.05, `lattice edge distance was ${latticeEdge}`);
  assert.ok(center > 0.4, `center distance was ${center}, expected ~inradius ${inradius}`);
  assert.ok(center > latticeEdge * 4, "center must be much farther from an edge than an edge sample");
});

test("neighbor hex center is also far from an edge", () => {
  const [x, y] = hexCenterCart(1, 0, size);
  const d = hexEdgeDistance(x, y, size);
  assert.ok(d > 0.4, `neighbor center distance ${d}`);
});

test("shipped field shader uses hex SDF, not dual-mod Voronoi", () => {
  const src = readFileSync(
    join(dirname(fileURLToPath(import.meta.url)), "../src/lib/silk.ts"),
    "utf8",
  );
  assert.match(HEX_SDF_GLSL, /sdHexagon/);
  assert.match(HEX_SDF_GLSL, /hexEdgeDistance/);
  assert.match(HEX_SDF_GLSL, /cubeRound/);
  assert.match(src, /HEX_SDF_GLSL/);
  assert.match(src, /hexEdgeDistance/);
  assert.doesNotMatch(src, /mod\(p, s\)/);
  assert.doesNotMatch(src, /mod\(p \+ 0\.5/);
  assert.doesNotMatch(HEX_SDF_GLSL, /mod\(p,\s*s\)/);
});
