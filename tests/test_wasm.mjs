import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import {createHash} from 'node:crypto';
import {initSync,effect_catalog} from '../web/vendor/engine.mjs';
import {createEffect,nextEffect} from '../web/effects.mjs';
const bytes=fs.readFileSync('web/vendor/effects.wasm');
initSync({module:bytes});
const catalogue=JSON.parse(effect_catalog()).map(e=>e.name);
const palette={background:'#101318',foreground:'#ffeecc',blue:'#ff8800',cyan:'#eeaa44',green:'#88bb44',yellow:'#ffcc55'};
test('vendored assets match pinned provenance',()=>{const manifest=JSON.parse(fs.readFileSync('web/vendor/manifest.json'));for(const [name,entry] of Object.entries(manifest.files))assert.equal(createHash('sha256').update(fs.readFileSync('web/vendor/'+name)).digest('hex'),entry.sha256);});
test('all 37 effects rotate without omissions or consecutive repeats',()=>{assert.equal(catalogue.length,37);const bag=[];let previous='';for(let round=0;round<3;round++){const played=[];for(let i=0;i<37;i++){const effect=nextEffect(catalogue,bag,previous,()=>.5);assert.notEqual(effect,previous);played.push(effect);previous=effect;}assert.equal(new Set(played).size,37);}});
const text=Array.from({length:24},(_,i)=>('╭─ \uf233 '+i+' \uf04b Working agent · synthetic task ').padEnd(140,'.')).join('\n');
for(const name of catalogue)test(`CLI ${name} completes with coloured cells`,()=>{const effect=createEffect(text,name,palette);let count=0,coloured=false,frame;try{while((frame=effect.next())){assert.equal(frame.width,140);assert.equal(frame.height,24);coloured ||= frame.fg.some(c=>c>>>24);assert.ok(++count<30000, 'effect must complete');}assert.ok(count>1);assert.ok(coloured);}finally{effect.free();effect.free();}assert.equal(effect.next(),null);});

test('bundled Nerd Font matches pinned source',()=>{assert.equal(createHash('sha256').update(fs.readFileSync('web/fonts/JetBrainsMonoNerdFont-Regular.ttf')).digest('hex'),'1c680e8cde9fcf8b88a5605ce8d1fb94dd3fb15841f7ca7bf4c55664855e5611');});
