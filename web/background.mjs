// Independently authored pixel field. Bayer ordered dithering is a standard technique.
const BAYER=[0,8,2,10,12,4,14,6,3,11,1,9,15,7,13,5];
const clamp=value=>Math.max(0,Math.min(1,Number.isFinite(value)?value:0));
export function musicEnergy(music,now=Date.now(),received=now){
  if(music?.state!=='playing' || !Number.isFinite(music.captured_at) || now-music.captured_at*1000<0 || now-music.captured_at*1000>3000 || now-received<0 || now-received>3000)return [];
  return Array.isArray(music.bands)?music.bands.slice(0,64).map(clamp):[];
}
export function gridFor(width,height){
  const spacing=Math.max(14,Math.ceil(Math.sqrt(Math.max(0,width)*Math.max(0,height)/12000)));
  return {spacing,columns:Math.ceil(Math.max(0,width)/spacing),rows:Math.ceil(Math.max(0,height)/spacing)};
}
export function ditherThreshold(x,y){return (BAYER[(y&3)*4+(x&3)]+.5)/16;}
export class PixelField {
  constructor(canvas,options={}){
    this.canvas=canvas;this.ctx=canvas.getContext('2d');this.environment=options.environment || globalThis;
    this.motion=this.environment.matchMedia?.('(prefers-reduced-motion: reduce)') || {matches:false};
    this.colours={background:'#101318',foreground:'#c0caf5',green:'#9ece6a',accent:'#7aa2f7'};
    this.music=null;this.received=0;this.position=null;this.impulses=[];this.lastFrame=-Infinity;this.lastNow=null;this.phase=0;this.disposed=false;
    this.resize();
  }
  update({colours,music}={}){
    for(const [key,value] of Object.entries(colours || {}))if(['background','foreground','green','accent','blue'].includes(key) && /^#[0-9a-f]{6}$/i.test(value))this.colours[key]=value;
    if(!colours?.accent && /^#[0-9a-f]{6}$/i.test(colours?.blue || ''))this.colours.accent=colours.blue;
    if(music!==undefined){this.music=music?{state:music.state,captured_at:music.captured_at,bands:Array.isArray(music.bands)?music.bands.slice(0,64):[]}:null;this.received=Date.now();}
  }
  resize(){
    const rect=this.canvas.getBoundingClientRect();this.width=Math.max(0,rect.width);this.height=Math.max(0,rect.height);
    this.ratio=Math.max(1,Math.min(this.environment.devicePixelRatio || 1,2));
    this.canvas.width=Math.round(this.width*this.ratio);this.canvas.height=Math.round(this.height*this.ratio);
    this.ctx?.setTransform(this.ratio,0,0,this.ratio,0,0);this.grid=gridFor(this.width,this.height);this.lastFrame=-Infinity;
  }
  pointer(x,y){this.position=Number.isFinite(x)&&Number.isFinite(y)?{x,y}:null;}
  click(x,y){
    if(this.disposed || this.motion.matches || this.environment.document?.hidden || !Number.isFinite(x) || !Number.isFinite(y))return;
    this.impulses.push({x,y,born:this.environment.performance?.now() ?? performance.now()});this.impulses=this.impulses.slice(-6);
  }
  frame(now){
    if(this.disposed || !this.ctx || !Number.isFinite(now))return;
    if(this.environment.document?.hidden){this.impulses=[];this.lastNow=null;return;}
    if(now-this.lastFrame<1000/30)return;
    const reduced=this.motion.matches;
    const delta=this.lastNow===null?0:Math.max(0,Math.min(100,now-this.lastNow));
    this.lastNow=now;this.lastFrame=now;
    if(!reduced)this.phase+=delta/1000;else this.impulses=[];
    this.impulses=this.impulses.filter(p=>now-p.born>=0 && now-p.born<1200);
    const bands=reduced?[]:musicEnergy(this.music,Date.now(),this.received);
    const {spacing,columns,rows}=this.grid,ctx=this.ctx;
    ctx.clearRect(0,0,this.width,this.height);
    const phase=reduced?0:this.phase;
    for(let row=0;row<rows;row++)for(let col=0;col<columns;col++){
      const x=col*spacing,y=row*spacing;
      const edge=Math.max(0,1-Math.min(x,this.width-x,y,this.height-y)/180);
      const wave=(Math.sin(col*.19+phase*.17)+Math.cos(row*.27-phase*.12)+2)/4;
      const band=bands.length?bands[Math.min(bands.length-1,Math.floor(col/Math.max(1,columns)*bands.length))]:0;
      const pointer=!reduced && this.position?Math.max(0,1-Math.hypot(x-this.position.x,y-this.position.y)/170):0;
      let ripple=0;
      if(!reduced)for(const impulse of this.impulses){const age=(now-impulse.born)/1200,radius=age*240;ripple+=Math.max(0,1-Math.abs(Math.hypot(x-impulse.x,y-impulse.y)-radius)/24)*(1-age);}
      const strength=clamp(.05+edge*.17+wave*.08+band*.25+pointer*.14+ripple*.28);
      if(strength<ditherThreshold(col,row))continue;
      ctx.globalAlpha=Math.min(.3,.06+edge*.09+band*.09+pointer*.06+ripple*.1);
      ctx.fillStyle=band>.1?this.colours.green:(this.colours.accent || this.colours.blue || this.colours.foreground);
      ctx.fillRect(x,y,2,2);
    }
    ctx.globalAlpha=1;
  }
  dispose(){this.disposed=true;this.impulses=[];this.music=null;this.position=null;this.ctx?.clearRect(0,0,this.width,this.height);}
}
