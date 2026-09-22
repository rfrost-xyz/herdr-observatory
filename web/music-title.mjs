import {titleColour} from './title.mjs';
const defaults={background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',cyan:'#7dcfff',green:'#9ece6a',yellow:'#e0af68'};
const bounded=value=>Array.from(String(value || '').replace(/[\x00-\x1f\x7f-\x9f]/g,' ')).slice(0,110).join('');
// Track changes only. Initial connection and recovery establish a new baseline.
export class MusicTitle {
  constructor(canvas,text,options={}) {
    this.field=options.field==='artist'?'artist':'title';this.canvas=canvas;this.text=text;this.ctx=canvas.getContext('2d');this.env=options.environment || globalThis;
    this.motion=this.env.matchMedia?.('(prefers-reduced-motion: reduce)') || {matches:false};
    this.load=options.load || (async()=>{const m=await import('./effects.mjs');return {...await m.loadEffects(),next:m.nextEffect};});
    this.palette={...defaults};this.bag=[];this.previous='';this.key=null;this.current=null;this.pending=null;this.session=null;this.cells=null;this.loading=false;this.generation=0;this.lastFrame=-Infinity;this.disposed=false;
    this.onVisibility=()=>{this.key=null;this.cancel();};this.env.document?.addEventListener?.('visibilitychange',this.onVisibility);this.motion.addEventListener?.('change',this.onVisibility);this.cancel();
  }
  update(track,palette={}) {
    for(const [k,v] of Object.entries(palette))if(k in defaults && /^#[0-9a-f]{6}$/i.test(v))this.palette[k]=v;
    if(!track || this.env.document?.hidden || this.motion.matches){this.key=null;this.current=null;this.cancel();return;}
    const item={key:JSON.stringify([track.title || '',track.artist || '']),text:bounded(track[this.field])};
    const changed=this.key!==null && item.key!==this.key;this.key=item.key;this.current=item;
    if(changed && item.text){if(this.session || this.loading)this.pending=item;else this.trigger(item);}
    this.draw();
  }
  activate(){if(this.current?.text)this.trigger(this.current);}
  cancel(){this.generation++;this.session?.free();this.session=null;this.cells=null;this.pending=null;this.canvas.hidden=true;this.text.style.visibility='visible';}
  async trigger(item) {
    if(this.disposed || !this.ctx || this.loading || this.session || this.motion.matches || this.env.document?.hidden)return;
    this.loading=true;const generation=this.generation;
    try {const lib=await this.load();if(this.disposed || generation!==this.generation || this.motion.matches || this.env.document?.hidden || this.key!==item.key)return;
      const name=lib.next(lib.catalogue,this.bag,this.previous);this.session=lib.create(item.text,name,this.palette);this.effectKey=item.key;this.cells=this.session.next();this.previous=name;if(!this.cells)this.finish();
    }catch{this.cancel();}finally{this.loading=false;}this.draw();
  }
  finish(){this.session?.free();this.session=null;this.cells=null;this.canvas.hidden=true;this.text.style.visibility='visible';}
  frame(now) {
    if(this.disposed)return;
    if(this.motion.matches || this.env.document?.hidden){this.key=null;this.current=null;this.cancel();return;}
    if(!Number.isFinite(now) || now-this.lastFrame<1000/30)return;this.lastFrame=now;
    if(this.session){try{this.cells=this.session.next();if(!this.cells)this.finish();}catch{this.cancel();}}
    if(!this.session && !this.loading && this.pending){const item=this.pending;this.pending=null;if(item.key===this.key)this.trigger(item);}
    this.draw();
  }
  draw(){
    const visible=Boolean(this.ctx && this.cells && this.effectKey===this.key);
    this.canvas.hidden=!visible;this.text.style.visibility=visible?'hidden':'visible';if(!visible)return;
    const box=this.text.getBoundingClientRect(),ratio=Math.min(2,this.env.devicePixelRatio || 1),ctx=this.ctx;
    this.canvas.width=Math.round(box.width*ratio);this.canvas.height=Math.round(box.height*ratio);ctx.setTransform(ratio,0,0,ratio,0,0);ctx.clearRect(0,0,box.width,box.height);
    const font=parseFloat(this.env.getComputedStyle?.(this.text)?.fontSize || '18');ctx.font=`${font}px "Observatory Nerd",monospace`;ctx.textBaseline='middle';
    const cell=ctx.measureText?.('M').width || font*.6,frame=this.cells;
    for(let i=0;i<frame.symbols.length;i++){
      const flag=frame.flags[i];if(flag&32)continue;const x=i*cell;if(x>=box.width)break;
      let fg=titleColour(frame.fg[i],this.palette.blue,this.palette),bg=titleColour(frame.bg[i],this.palette.background,this.palette);
      if(flag&16)[fg,bg]=[bg,fg];ctx.globalAlpha=flag&2?.55:1;if(bg!==this.palette.background){ctx.fillStyle=bg;ctx.fillRect(x,0,cell,box.height);}
      ctx.fillStyle=fg;const code=frame.symbols[i];if(code && code!==32)ctx.fillText(String.fromCodePoint(code),x,box.height/2,cell);
    }ctx.globalAlpha=1;
  }
  dispose(){this.disposed=true;this.env.document?.removeEventListener?.('visibilitychange',this.onVisibility);this.motion.removeEventListener?.('change',this.onVisibility);this.cancel();}
}
