/** Flat-top hex lattice. Same math as the field fragment shader. */

export const SQRT3 = Math.sqrt(3);

/** IQ sdHexagon: r is inradius (distance center → flat). Negative inside. */
export function sdHexagon(px: number, py: number, r: number): number {
  const kx = -SQRT3 / 2;
  const ky = 0.5;
  const kz = 1 / SQRT3;
  let x = Math.abs(px);
  let y = Math.abs(py);
  const t = 2 * Math.min(kx * x + ky * y, 0);
  x -= t * kx;
  y -= t * ky;
  x -= Math.max(-kz * r, Math.min(x, kz * r));
  y -= r;
  const len = Math.hypot(x, y);
  return len * Math.sign(y);
}

export function cubeRound(q: number, r: number): [number, number] {
  let x = q;
  let z = r;
  let y = -x - z;
  let rx = Math.round(x);
  let ry = Math.round(y);
  let rz = Math.round(z);
  const xd = Math.abs(rx - x);
  const yd = Math.abs(ry - y);
  const zd = Math.abs(rz - z);
  if (xd > yd && xd > zd) rx = -ry - rz;
  else if (yd > zd) ry = -rx - rz;
  else rz = -rx - ry;
  return [rx, rz];
}

/** Flat-top: size is circumradius. */
export function axialFromCart(x: number, y: number, size: number): [number, number] {
  const q = ((2 / 3) * x) / size;
  const rr = ((-1 / 3) * x + (SQRT3 / 3) * y) / size;
  return [q, rr];
}

export function hexCenterCart(q: number, r: number, size: number): [number, number] {
  const x = size * (1.5 * q);
  const y = size * ((SQRT3 / 2) * q + SQRT3 * r);
  return [x, y];
}

/** Distance to the nearest hex edge. ~0 on edges, ~inradius at cell centers. */
export function hexEdgeDistance(x: number, y: number, size: number): number {
  const [q, r] = axialFromCart(x, y, size);
  const [qq, rr] = cubeRound(q, r);
  const [cx, cy] = hexCenterCart(qq, rr, size);
  const inradius = (SQRT3 / 2) * size;
  return Math.abs(sdHexagon(x - cx, y - cy, inradius));
}

export const HEX_SDF_GLSL = `
float sdHexagon(vec2 p, float r) {
  const vec3 k = vec3(-0.866025404, 0.5, 0.577350269);
  p = abs(p);
  p -= 2.0 * min(dot(k.xy, p), 0.0) * k.xy;
  p -= vec2(clamp(p.x, -k.z * r, k.z * r), r);
  return length(p) * sign(p.y);
}

vec2 cubeRound(vec2 qr) {
  vec3 c = vec3(qr.x, -qr.x - qr.y, qr.y);
  vec3 rc = floor(c + 0.5);
  vec3 d = abs(rc - c);
  if (d.x > d.y && d.x > d.z) rc.x = -rc.y - rc.z;
  else if (d.y > d.z) rc.y = -rc.x - rc.z;
  else rc.z = -rc.x - rc.y;
  return vec2(rc.x, rc.z);
}

vec2 axialFromCart(vec2 p, float size) {
  float q = (2.0 / 3.0 * p.x) / size;
  float r = (-1.0 / 3.0 * p.x + 0.577350269 * p.y) / size;
  return vec2(q, r);
}

vec2 hexCenterCart(vec2 qr, float size) {
  float x = size * (1.5 * qr.x);
  float y = size * (0.866025404 * qr.x + 1.732050808 * qr.y);
  return vec2(x, y);
}

float hexEdgeDistance(vec2 p, float size) {
  vec2 qr = cubeRound(axialFromCart(p, size));
  vec2 c = hexCenterCart(qr, size);
  float inr = 0.866025404 * size;
  return abs(sdHexagon(p - c, inr));
}
`;
