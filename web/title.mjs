// Delta Corps Priest 1 subset; attribution in vendor/STAMPS-NOTICE.
const GLYPHS={"R": ["   ▄████████", "  ███    ███", "  ███    ███", " ▄███▄▄▄▄██▀", "▀▀███▀▀▀▀▀", "▀███████████", "  ███    ███", "  ███    ███"], "I": [" ▄█", "███", "███▌", "███▌", "███▌", "███", "███", "█▀"], "C": [" ▄████████", "███    ███", "███    █▀", "███", "███", "███    █▄", "███    ███", "████████▀"], "H": ["   ▄█    █▄", "  ███    ███", "  ███    ███", " ▄███▄▄▄▄███▄▄", "▀▀███▀▀▀▀███▀", "  ███    ███", "  ███    ███", "  ███    █▀"]};
export const TITLE_ROWS=Array.from({length:8},(_,row)=>Array.from('RICH').map(letter=>GLYPHS[letter][row].padEnd(Math.max(...GLYPHS[letter].map(x=>Array.from(x).length)),' ')).join(' '));
export const TITLE_TEXT=TITLE_ROWS.join('\n');
const defaults={background:'#101318',foreground:'#c0caf5',blue:'#7aa2f7',cyan:'#7dcfff',green:'#9ece6a',yellow:'#e0af68'};
export function titleColour(value,fallback,palette){
  if(!(value>>>24))return fallback;
  const rgb=[value>>>16&255,value>>>8&255,value&255],intensity=Math.max(...rgb)/255;
  const normal=v=>{const maximum=Math.max(...v)||1;return v.map(c=>c/maximum);};
  const target=normal(rgb),candidates=['blue','cyan','green','yellow','foreground'].map(key=>palette[key].slice(1).match(/../g).map(x=>parseInt(x,16)));
  let best=0,score=Infinity;candidates.forEach((v,i)=>{const distance=normal(v).reduce((sum,c,j)=>sum+(c-target[j])**2,0);if(distance<score){score=distance;best=i;}});
  return `rgb(${(score<.0001?rgb:candidates[best].map(c=>Math.round(c*intensity))).join(',')})`;
}
export class TitleMark {
  constructor(canvas,options={}){
    this.canvas=canvas;this.ctx=canvas.getContext('2d');this.env=options.environment || globalThis;this.motion=this.env.matchMedia?.('(prefers-reduced-motion: reduce)') || {matches:false};
    this.load=options.load || (async()=>{const module=await import('./effects.mjs');return {...await module.loadEffects(),next:module.nextEffect};});
    this.palette={...defaults};this.bag=[];this.previous='';this.session=null;this.cells=null;this.loading=false;this.generation=0;this.disposed=false;this.lastFrame=-Infinity;this.nextAt=this.now()+60000;this.onVisibility=()=>{this.cancel();this.draw();};this.env.document?.addEventListener?.('visibilitychange',this.onVisibility);this.motion.addEventListener?.('change',this.onVisibility);this.resize();
  }
  now(){return this.env.performance?.now() ?? performance.now();}
  update(palette){for(const [key,value] of Object.entries(palette || {}))if(key in defaults && /^#[0-9a-f]{6}$/i.test(value))this.palette[key]=value;}
  resize(){const box=this.canvas.getBoundingClientRect();this.width=box.width;this.height=box.height;const ratio=Math.max(1,Math.min(2,this.env.devicePixelRatio || 1));this.canvas.width=Math.round(this.width*ratio);this.canvas.height=Math.round(this.height*ratio);this.ctx?.setTransform(ratio,0,0,ratio,0,0);this.draw();}
  cancel(now=this.now(),resetSchedule=true){this.generation++;this.session?.free();this.session=null;this.cells=null;if(resetSchedule)this.nextAt=now+60000;}
  async trigger(now=this.now()){
    if(this.disposed || !this.ctx || this.loading || this.session || this.env.document?.hidden || this.motion.matches)return;
    this.loading=true;const generation=this.generation;this.nextAt=now+60000;
    try{const library=await this.load();if(this.disposed || generation!==this.generation || this.env.document?.hidden || this.motion.matches)return;
      const name=library.next(library.catalogue,this.bag,this.previous);this.session=library.createTitle(TITLE_TEXT,name,this.palette);this.cells=this.session.next();this.previous=name;if(!this.cells)this.cancel(now,false);this.draw();
    }catch{this.cancel();}finally{this.loading=false;}
  }
  frame(now){
    if(this.disposed || !Number.isFinite(now))return;
    if(this.env.document?.hidden || this.motion.matches){this.cancel(now);if(!this.env.document?.hidden)this.draw();return;}
    if(now-this.lastFrame<1000/30)return;this.lastFrame=now;
    if(this.session){try{const next=this.session.next();if(next)this.cells=next;else this.cancel(now,false);}catch{this.cancel(now);}}
    else if(!this.loading && now>=this.nextAt)this.trigger(now);
    this.draw();
  }
  draw(){
    const ctx=this.ctx;if(!ctx)return;ctx.clearRect(0,0,this.width,this.height);
    const columns=Array.from(TITLE_ROWS[0]).length,cell=this.width/columns,line=this.height/8;
    const fontSize=Math.min(cell/.6,line),baseFont=`${fontSize}px "Observatory Nerd",monospace`;ctx.textBaseline='top';
    const cells=this.cells;
    for(let row=0;row<8;row++)for(let col=0;col<columns;col++){
      const i=row*columns+col,flag=cells?.flags[i] || 0;if(flag&32)continue;
      const code=cells?cells.symbols[i]:TITLE_ROWS[row].codePointAt(col),x=col*cell,y=row*line;
      let fg=titleColour(cells?.fg[i],this.palette.foreground,this.palette),bg=titleColour(cells?.bg?.[i],this.palette.background,this.palette);
      if(flag&16)[fg,bg]=[bg,fg];ctx.globalAlpha=flag&2?.55:1;
      if(bg!==this.palette.background){ctx.fillStyle=bg;ctx.fillRect(x,y,cell,line);}
      ctx.fillStyle=fg;ctx.font=(flag&4?'italic ':'')+(flag&1?'bold ':'')+baseFont;
      if(code && code!==32)ctx.fillText(String.fromCodePoint(code),x,y,cell);
      if(flag&8)ctx.fillRect(x,y+line*.85,cell,Math.max(.5,line*.08));
      if(flag&64)ctx.fillRect(x,y+line*.5,cell,Math.max(.5,line*.08));
    }ctx.globalAlpha=1;
  }
  dispose(){this.env.document?.removeEventListener?.('visibilitychange',this.onVisibility);this.motion.removeEventListener?.('change',this.onVisibility);this.cancel();this.disposed=true;this.ctx?.clearRect(0,0,this.width,this.height);}
}
