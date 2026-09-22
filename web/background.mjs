// Direct adaptation of Omarchy HeroPixelField.tsx. See vendor/OMARCHY-BACKGROUND-NOTICE.
const BAYER = [
  0, 32, 8, 40, 2, 34, 10, 42, 48, 16, 56, 24, 50, 18, 58, 26, 12, 44, 4, 36,
  14, 46, 6, 38, 60, 28, 52, 20, 62, 30, 54, 22, 3, 35, 11, 43, 1, 33, 9, 41,
  51, 19, 59, 27, 49, 17, 57, 25, 15, 47, 7, 39, 13, 45, 5, 37, 63, 31, 55, 23,
  61, 29, 53, 21,
]

const NOISE_SIZE = 128
/** Grid cells per unit of noise: how big the drifting blobs read. */
const CELLS_PER_NOISE = 9
/** Cursor reach, in grid cells. */
const CURSOR_CELLS = 12

/** How much of the field's height the loudest band may climb. */
const SPECTRUM_REACH = 0.92
/** How dense a column gets, and how much of it wears the main ink. */
const SPECTRUM_DENSITY = 0.7
const SPECTRUM_HEAT = 0.5
/** Below this a band is resting and its column shows nothing extra. */
const SPECTRUM_FLOOR = 0.08
const LOGO_SIZE = 15
const LOGO_ROWS = [
  '111111111111111',
  '100000010000001',
  '101111110001101',
  '101000000000101',
  '101000000000101',
  '101000000000101',
  '101000000000101',
  '111000000000101',
  '101000000000101',
  '101000000000101',
  '101000000000101',
  '101000000000101',
  '101111111111101',
  '100000010000001',
  '111111110111111',
]


function buildNoise(seed) {
  const size = NOISE_SIZE
  let state = seed >>> 0
  const random = () => {
    state = (state * 1664525 + 1013904223) >>> 0
    return state / 4294967296
  }

  let field = new Float32Array(size * size)
  for (let i = 0; i < field.length; i++) field[i] = random()

  // A couple of box passes turn white noise into soft blobs.
  for (let pass = 0; pass < 2; pass++) {
    const next = new Float32Array(size * size)
    for (let y = 0; y < size; y++) {
      for (let x = 0; x < size; x++) {
        let sum = 0
        for (let dy = -1; dy <= 1; dy++) {
          for (let dx = -1; dx <= 1; dx++) {
            const sx = (x + dx + size) % size
            const sy = (y + dy + size) % size
            sum += field[sy * size + sx]
          }
        }
        next[y * size + x] = sum / 9
      }
    }
    field = next
  }

  // Box blurring collapses the range, so stretch it back out.
  let min = Infinity
  let max = -Infinity
  for (const v of field) {
    if (v < min) min = v
    if (v > max) max = v
  }
  const span = max - min || 1
  for (let i = 0; i < field.length; i++) field[i] = (field[i] - min) / span

  return field
}

/** A fixed 64x64 tile of per-cell threshold offsets, tiled over the grid. */
function buildJitter(seed) {
  let state = seed >>> 0
  const tile = new Float32Array(64 * 64)
  for (let i = 0; i < tile.length; i++) {
    state = (state * 1664525 + 1013904223) >>> 0
    tile[i] = state / 4294967296
  }
  return tile
}

function sample(field, x, y) {
  const size = NOISE_SIZE
  const xi = Math.floor(x)
  const yi = Math.floor(y)
  const fx = x - xi
  const fy = y - yi
  const x0 = ((xi % size) + size) % size
  const y0 = ((yi % size) + size) % size
  const x1 = (x0 + 1) % size
  const y1 = (y0 + 1) % size
  const sx = fx * fx * (3 - 2 * fx)
  const sy = fy * fy * (3 - 2 * fy)
  const a = field[y0 * size + x0]
  const b = field[y0 * size + x1]
  const c = field[y1 * size + x0]
  const d = field[y1 * size + x1]
  return (a * (1 - sx) + b * sx) * (1 - sy) + (c * (1 - sx) + d * sx) * sy
}

const clamp=value=>Math.max(0,Math.min(1,Number.isFinite(value)?value:0));
export function musicEnergy(music,now=Date.now(),received=now){
  if(music?.state!=='playing' || !Number.isFinite(music.captured_at) || now-music.captured_at*1000<0 || now-music.captured_at*1000>3000 || now-received<0 || now-received>3000)return [];
  return Array.isArray(music.bands)?music.bands.slice(0,64).map(clamp):[];
}
export function gridFor(width,height){
  const spacing=Math.max(8,Math.ceil(Math.sqrt(Math.max(0,width)*Math.max(0,height)/12000)));
  return {spacing,columns:Math.ceil(Math.max(0,width)/spacing),rows:Math.ceil(Math.max(0,height)/spacing)};
}

export function ditherThreshold(x,y){return (BAYER[(y&7)*8+(x&7)]+.5)/64;}
const mix=(from,to,t)=>{const a=from.slice(1).match(/../g).map(x=>parseInt(x,16)),b=to.slice(1).match(/../g).map(x=>parseInt(x,16));return `rgb(${a.map((v,i)=>Math.round(v+(b[i]-v)*t)).join(',')})`;};
export class PixelField {
  constructor(canvas,options={}){
    this.canvas=canvas;this.ctx=canvas.getContext('2d');this.environment=options.environment || globalThis;
    this.motion=this.environment.matchMedia?.('(prefers-reduced-motion: reduce)') || {matches:false};
    this.colours={background:'#101318',foreground:'#c0caf5',green:'#9ece6a',accent:'#7aa2f7'};
    this.noise=buildNoise(0x9ece6a);this.jitter=buildJitter(0x0a1f14);this.bandsNow=new Float32Array(32);
    this.music=null;this.received=0;this.position=null;this.impulses=[];this.lastFrame=-Infinity;this.lastNow=null;this.phase=0;this.strength=0;this.beatPulse=0;this.disposed=false;this.resize();
  }
  update({colours,music}={}){
    for(const [key,value] of Object.entries(colours || {}))if(['background','foreground','green','accent','blue'].includes(key) && /^#[0-9a-f]{6}$/i.test(value))this.colours[key]=value;
    if(!colours?.accent && /^#[0-9a-f]{6}$/i.test(colours?.blue || ''))this.colours.accent=colours.blue;
    if(music!==undefined){this.music=music?{state:music.state,captured_at:music.captured_at,bands:Array.isArray(music.bands)?music.bands.slice(0,64):[],beat:clamp(music.beat)}:null;this.received=Date.now();}
  }
  resize(){
    const rect=this.canvas.getBoundingClientRect();this.width=Math.max(0,rect.width);this.height=Math.max(0,rect.height);this.ratio=Math.max(1,Math.min(this.environment.devicePixelRatio || 1,2));
    this.canvas.width=Math.round(this.width*this.ratio);this.canvas.height=Math.round(this.height*this.ratio);this.ctx?.setTransform(this.ratio,0,0,this.ratio,0,0);this.grid=gridFor(this.width,this.height);this.lastFrame=-Infinity;
  }
  pointer(x,y){this.position=Number.isFinite(x)&&Number.isFinite(y)?{x,y}:null;}
  click(x,y){
    if(this.disposed || this.motion.matches || this.environment.document?.hidden || !Number.isFinite(x) || !Number.isFinite(y))return;
    // Upstream launch at quick-click charge zero; the public API has no held press.
    const from=.45;
    this.impulses=[...this.impulses.slice(-3),{x,y,born:this.environment.performance?.now() ?? performance.now(),from,to:(from+1)*(.92+Math.random()*.16),life:.65*(.92+Math.random()*.16)}];
  }
  frame(now){
    if(this.disposed || !this.ctx || !Number.isFinite(now))return;
    if(this.environment.document?.hidden){this.impulses=[];this.bandsNow.fill(0);this.beatPulse=0;this.lastNow=null;return;}
    if(now-this.lastFrame<1000/30)return;
    const reduced=this.motion.matches,delta=this.lastNow===null?0:Math.max(0,Math.min(100,now-this.lastNow));this.lastNow=now;this.lastFrame=now;if(!reduced)this.phase+=delta/1000;else this.impulses=[];
    const heard=reduced?[]:musicEnergy(this.music,Date.now(),this.received);
    const BANDS=this.bandsNow.length,bandsNow=this.bandsNow;
    let listening=false;
    if(!heard.length){bandsNow.fill(0);this.beatPulse=0;}
    else {for(let i=0;i<BANDS;i++){const pos=i*(heard.length-1)/Math.max(1,BANDS-1),lo=Math.floor(pos),value=heard[lo]*(1-(pos-lo))+heard[Math.min(heard.length-1,lo+1)]*(pos-lo);bandsNow[i]+=(value-bandsNow[i])*(value>bandsNow[i]?.7:.14);if(bandsNow[i]>.01)listening=true;}
      // Only an explicitly captured beat drives the upstream beat envelope.
      this.beatPulse=Math.max(this.beatPulse*.84,this.music.beat);if(this.beatPulse<.005)this.beatPulse=0;}
    const {spacing:wmCW,columns:cols,rows}=this.grid,wmCH=wmCW,wmX=0,wmY=0,cMin=0,rMin=0;
    const ramp=new Float32Array(rows*cols).fill(.3),noise=this.noise,jitter=this.jitter,t=reduced?0:this.phase,ctx=this.ctx;
    const palette={bg:this.colours.background,dim:mix(this.colours.background,this.colours.green,.27),mid:mix(this.colours.background,this.colours.green,.58),lit:this.colours.green};
    ctx.globalAlpha=1;ctx.clearRect(0,0,this.width,this.height);
    this.strength=reduced?0:this.strength+((this.position?1:0)-this.strength)*.3;
    const glows=this.position && this.strength>.01?[{...this.position,strength:this.strength,reach:CURSOR_CELLS*wmCW*(.45+.55*this.strength)*(1+.8*this.beatPulse)}]:[];
    this.impulses=this.impulses.filter(p=>now>=p.born && (now-p.born)/1000<p.life);
    const stamps=this.impulses.map(p=>{const age=(now-p.born)/1000/p.life,grow=1-(1-age)**3;return {x:p.x,y:p.y,cellPx:wmCW*(p.from+(p.to-p.from)*grow),amp:(1-age)**1.7};});
    const stampAt=(cx,cy)=>{let amp=0;for(const stamp of stamps){const lx=Math.floor((cx-stamp.x)/stamp.cellPx+LOGO_SIZE/2),ly=Math.floor((cy-stamp.y)/stamp.cellPx+LOGO_SIZE/2);if(lx<0 || ly<0 || lx>=LOGO_SIZE || ly>=LOGO_SIZE)continue;if(LOGO_ROWS[ly][lx]==='1' && stamp.amp>amp)amp=stamp.amp;}return amp;};
      for (let r = 0; r < rows; r++) {
        const row = rMin + r
        const yTop = wmY + row * wmCH
        const y = Math.round(yTop)
        const cellH = Math.round(yTop + wmCH) - y
        const cy = yTop + wmCH / 2
        for (let c = 0; c < cols; c++) {
          const col = cMin + c
          const shade = ramp[r * cols + c]
          let lum = 0

          if (shade > 0.002) {
            const u = col / CELLS_PER_NOISE
            const v = row / CELLS_PER_NOISE
            const base =
              0.6 * sample(noise, u + t * 0.14, v - t * 0.055) +
              0.4 * sample(noise, u * 0.55 - t * 0.08, v * 0.55 + t * 0.06)

            const twinkle =
              0.5 +
              0.5 *
                Math.sin(t * 1.1 + jitter[(row * 37 + col * 11) & 4095] * 6.283)

            lum = shade * (0.3 + 0.52 * base * base + 0.18 * twinkle) * 0.62
          }

          const xLeft = wmX + col * wmCW
          const cx = xLeft + wmCW / 2

          let glowAmount = 0
          for (const glow of glows) {
            const dx = cx - glow.x
            const dy = cy - glow.y
            const dist = Math.sqrt(dx * dx + dy * dy)
            if (dist < glow.reach) {
              const falloff = 1 - dist / glow.reach
              const amount = falloff * falloff * glow.strength
              if (amount > glowAmount) glowAmount = amount
            }
          }
          lum += glowAmount * 0.6

          let waveAmount = 0
          if (stamps.length > 0) {
            waveAmount = stampAt(cx, cy)
            lum += waveAmount * 1.15
          }

          let specAmount = 0
          if (listening && shade > 0.002) {
            const across = (c + 0.5) / cols
            const side = Math.abs(across - 0.5) * 2
            const bandPos = (1 - side) * BANDS - 0.5
            const b0 = Math.max(0, Math.min(BANDS - 1, Math.floor(bandPos)))
            const b1 = Math.min(BANDS - 1, b0 + 1)
            const mixB = Math.max(0, Math.min(1, bandPos - b0))
            const raw = bandsNow[b0] * (1 - mixB) + bandsNow[b1] * mixB
            const level = Math.max(
              0,
              (raw - SPECTRUM_FLOOR) / (1 - SPECTRUM_FLOOR),
            )
            const fromBottom = rows - 1 - r
            const tall = level * rows * SPECTRUM_REACH
            if (level > 0 && fromBottom < tall) {
              specAmount = level * (1 - fromBottom / tall) ** 0.85
              lum += specAmount * SPECTRUM_DENSITY * Math.min(1, shade * 3)
            }
          }

          // Pure Bayer would light the same low-index cells everywhere and
          // read as a regular lattice at this density, so a fixed per-cell
          // offset scatters the resting field while the ordered structure
          // still shows up where the cursor pushes luminance high.
          const threshold =
            0.78 * ((BAYER[(row & 7) * 8 + (col & 7)] + 0.5) / 64) +
            0.22 * jitter[(row & 63) * 64 + (col & 63)]
          if (lum <= threshold) continue

          const heat = Math.max(
            glowAmount,
            waveAmount,
            specAmount * SPECTRUM_HEAT,
          )
          ctx.fillStyle =
            heat > 0.34 ? palette.lit : heat > 0.1 ? palette.mid : palette.dim
          const x = Math.round(xLeft)
          ctx.fillRect(x, y, Math.round(xLeft + wmCW) - x, cellH)
        }
      }


    ctx.globalAlpha=1;
  }
  dispose(){this.disposed=true;this.impulses=[];this.music=null;this.position=null;this.ctx?.clearRect(0,0,this.width,this.height);}
}
