import init, {Session, effect_catalog} from './vendor/engine.mjs';
let ready;
export async function loadEffects() {
  ready ??= init({module_or_path: new URL('./vendor/effects.wasm', import.meta.url)}).catch(error=>{ready=null;throw error;});
  await ready;
  return {catalogue: JSON.parse(effect_catalog()).map(effect=>effect.name), create: createEffect};
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
  const colours=[palette.blue,palette.cyan,palette.green,palette.yellow,palette.foreground].map(c=>c.slice(1)).join(',');
  if(typeof text!=='string' || !text.length || /[\r\n]/.test(text) || Array.from(text).length>138)throw new Error('Expected one bounded event line');
  const columns=Array.from(text).length;
  const session=new Session(text,name,columns,1,undefined,30,colours,palette.background.slice(1));
  let cells, freed=false;
  return {
    next(){
      if(freed)return null;
      if(!session.step())return null;
      const width=session.width(),height=session.height(),size=width*height;
      if(width!==columns || height!==1)throw new Error('Invalid effect dimensions');
      cells ??= {width,height,symbols:new Uint32Array(size),fg:new Uint32Array(size),bg:new Uint32Array(size),flags:new Uint8Array(size)};
      session.fill(cells.symbols,cells.fg,cells.bg,cells.flags);
      return cells;
    },
    free(){if(!freed){freed=true;session.free();}}
  };
}
