import init, {Session, effect_catalog} from './vendor/engine.mjs';
let ready;
export async function loadEffects() {
  ready ??= init({module_or_path: new URL('./vendor/effects.wasm', import.meta.url)}).catch(error=>{ready=null;throw error;});
  await ready;
  return {catalogue: JSON.parse(effect_catalog()).map(effect=>effect.name), create: createEffect, createTitle: createTitleEffect};
}
// Visit every effect once before reshuffling, including a different boundary pair.
export function nextEffect(catalogue, bag, previous, random=Math.random) {
  if(!bag.length) {
    bag.push(...catalogue);
    for(let i=bag.length-1;i>0;i--){const j=Math.floor(random()*(i+1));[bag[i],bag[j]]=[bag[j],bag[i]];}
    if(bag.length>1 && bag.at(-1)===previous)[bag[0],bag[bag.length-1]]=[bag.at(-1),bag[0]];
  }
  return bag.pop();
}
export function createEffect(text, name, palette) {
  if(typeof text!=='string' || !text.length || /[\r\n]/.test(text) || Array.from(text).length>138)throw new Error('Expected one bounded event line');
  const columns=Array.from(text).length;
  return createSession(text,name,palette,columns,1);
}
export function createTitleEffect(text,name,palette) {
  const rows=typeof text==='string'?text.split('\n'):[];
  const columns=Math.max(0,...rows.map(row=>Array.from(row).length));
  if(rows.length!==8 || columns<1 || columns>60 || rows.some(row=>/[\r\x00-\x1f]/.test(row)))throw new Error('Expected bounded eight-row title');
  return createSession(text,name,palette,columns,8,true);
}
function createSession(text,name,palette,columns,rows,settle=false) {
  const colours=[palette.blue,palette.cyan,palette.green,palette.yellow,palette.foreground].map(c=>c.slice(1)).join(',');
  const session=new Session(text,name,columns,rows,undefined,30,colours,palette.background.slice(1));
  let cells, freed=false, settled=false;
  return {
    next(){
      if(freed)return null;
      if(!session.step()){
        if(!settle || settled)return null;
        settled=true;
        const symbols=Uint32Array.from(text.split('\n').flatMap(row=>Array.from(row.padEnd(columns,' '),glyph=>glyph.codePointAt(0))));
        return {width:columns,height:rows,symbols,fg:new Uint32Array(columns*rows).fill((0xff000000 | parseInt(palette.foreground.slice(1),16))>>>0),bg:new Uint32Array(columns*rows),flags:new Uint8Array(columns*rows)};
      }
      const width=session.width(),height=session.height(),size=width*height;
      if(width!==columns || height!==rows)throw new Error('Invalid effect dimensions');
      cells ??= {width,height,symbols:new Uint32Array(size),fg:new Uint32Array(size),bg:new Uint32Array(size),flags:new Uint8Array(size)};
      session.fill(cells.symbols,cells.fg,cells.bg,cells.flags);
      return cells;
    },
    free(){if(!freed){freed=true;session.free();}}
  };
}
