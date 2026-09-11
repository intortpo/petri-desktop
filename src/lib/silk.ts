import { HEX_SDF_GLSL } from "./hex-lattice";

const VERT = `
attribute vec2 a_pos;
void main() {
  gl_Position = vec4(a_pos, 0.0, 1.0);
}
`;

export const FIELD_FRAG = `
precision highp float;
uniform vec2 u_res;
uniform float u_time;
uniform vec3 u_field;
uniform vec3 u_wire;
${HEX_SDF_GLSL}
void main() {
  vec2 uv = gl_FragCoord.xy / u_res.xy;
  float aspect = u_res.x / max(u_res.y, 1.0);
  vec2 p = (uv * 2.0 - 1.0) * vec2(aspect, 1.0);
  float t = u_time * 0.018;
  p += vec2(t * 0.15, t * 0.07);
  float size = 0.09;
  float e = hexEdgeDistance(p, size);
  float line = 1.0 - smoothstep(0.0, 0.0035, e);
  vec3 col = mix(u_field, u_wire, line * 0.055);
  gl_FragColor = vec4(col, 1.0);
}
`;

const FRAG = FIELD_FRAG;

function compile(gl: WebGLRenderingContext, type: number, src: string) {
  const sh = gl.createShader(type)!;
  gl.shaderSource(sh, src);
  gl.compileShader(sh);
  if (!gl.getShaderParameter(sh, gl.COMPILE_STATUS)) {
    throw new Error(gl.getShaderInfoLog(sh) || "shader compile failed");
  }
  return sh;
}

export type SilkColors = { field: [number, number, number]; wire: [number, number, number] };

export function startSilk(canvas: HTMLCanvasElement, colors: () => SilkColors): () => void {
  const gl = canvas.getContext("webgl", {
    antialias: false,
    alpha: false,
    depth: false,
    stencil: false,
    powerPreference: "low-power",
  });
  if (!gl) return () => {};

  const prog = gl.createProgram()!;
  gl.attachShader(prog, compile(gl, gl.VERTEX_SHADER, VERT));
  gl.attachShader(prog, compile(gl, gl.FRAGMENT_SHADER, FRAG));
  gl.linkProgram(prog);
  gl.useProgram(prog);

  const buf = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buf);
  gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([-1, -1, 1, -1, -1, 1, 1, 1]), gl.STATIC_DRAW);
  const loc = gl.getAttribLocation(prog, "a_pos");
  gl.enableVertexAttribArray(loc);
  gl.vertexAttribPointer(loc, 2, gl.FLOAT, false, 0, 0);

  const uRes = gl.getUniformLocation(prog, "u_res");
  const uTime = gl.getUniformLocation(prog, "u_time");
  const uField = gl.getUniformLocation(prog, "u_field");
  const uWire = gl.getUniformLocation(prog, "u_wire");

  let raf = 0;
  let alive = true;
  const t0 = performance.now();
  const reduced = window.matchMedia("(prefers-reduced-motion: reduce)");

  const resize = () => {
    const dpr = Math.min(window.devicePixelRatio || 1, 1.5);
    const w = Math.max(1, Math.floor(canvas.clientWidth * dpr));
    const h = Math.max(1, Math.floor(canvas.clientHeight * dpr));
    if (canvas.width !== w || canvas.height !== h) {
      canvas.width = w;
      canvas.height = h;
      gl.viewport(0, 0, w, h);
    }
  };

  const frame = (now: number) => {
    if (!alive) return;
    resize();
    const t = reduced.matches ? 0 : (now - t0) / 1000;
    const c = colors();
    gl.uniform2f(uRes, canvas.width, canvas.height);
    gl.uniform1f(uTime, t);
    gl.uniform3f(uField, c.field[0], c.field[1], c.field[2]);
    gl.uniform3f(uWire, c.wire[0], c.wire[1], c.wire[2]);
    gl.drawArrays(gl.TRIANGLE_STRIP, 0, 4);
    raf = requestAnimationFrame(frame);
  };

  raf = requestAnimationFrame(frame);
  return () => {
    alive = false;
    cancelAnimationFrame(raf);
  };
}
