import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
const root='omarchy/herdr.observatory';
test('distribution uses native commands without retired runtime assets',()=>{
  const installer=fs.readFileSync(`${root}/install.sh`,'utf8');
  const uninstaller=fs.readFileSync(`${root}/uninstall.sh`,'utf8');
  const panel=fs.readFileSync(`${root}/Panel.qml`,'utf8');
  const store=fs.readFileSync(`${root}/SnapshotStore.qml`,'utf8');
  const controller=fs.readFileSync(`${root}/AntonController.qml`,'utf8');
  for(const source of [installer,uninstaller,panel,controller,store,fs.readFileSync('hooks/observatory.ts','utf8')]){
    assert.doesNotMatch(source,/python3|runtime\.zip|native-adapter\.py|docker exec/);
  }
  assert.match(controller,/State\.navigationArgs\(entry\)/);
  assert.match(fs.readFileSync(`${root}/State.js`,'utf8'),/"--open-thread"/);
  for(const source of [panel,controller])assert.doesNotMatch(source,/--refresh-allowances|id: allowanceRefresh/);
  assert.match(store,/anton-runtime/);
  assert.ok(uninstaller.indexOf('--remove-peers')<uninstaller.indexOf('setPluginEnabled'));
});
test('every plugin QML file and State.js is installed and removed',()=>{
  const installer=fs.readFileSync(`${root}/install.sh`,'utf8');
  const uninstaller=fs.readFileSync(`${root}/uninstall.sh`,'utf8');
  const installed=installer.match(/^files=\((.*)\)$/m)[1].split(' ');
  const allowed=uninstaller.match(/^\s*(\.hooks-receipt\.json\|.*)\) ;;$/m)[1].split('|');
  const removed=uninstaller.match(/^for file in (.*); do$/m)[1].split(' ');
  const shipped=fs.readdirSync(root).filter(p=>p.endsWith('.qml')).concat(['State.js']);
  assert.ok(shipped.length>=17,shipped.join(' '));
  for(const file of shipped){
    assert.ok(installed.includes(file),`install.sh files: ${file}`);
    assert.ok(allowed.includes(file),`uninstall.sh allowlist: ${file}`);
    assert.ok(removed.includes(file),`uninstall.sh removal loop: ${file}`);
  }
});
test('supported application excludes legacy web and Python sources',()=>{
  for(const path of ['web','observatory','deploy','Dockerfile','package.json','package-lock.json',`${root}/runtime.py`,`${root}/runtime.zip`,`${root}/native-adapter.py`,`${root}/open-thread.py`,`${root}/build-native.py`,`${root}/build-runtime.py`])assert.ok(!fs.existsSync(path),path);
  for(const path of fs.readdirSync('hooks'))assert.ok(!path.endsWith('.py'),path);
  const source=fs.readdirSync('omarchy/anton-runtime/src').filter(p=>p.endsWith('.rs')).map(p=>fs.readFileSync(`omarchy/anton-runtime/src/${p}`,'utf8')).join('\n');
  assert.doesNotMatch(source,/Command::new\("(?:python3?|docker)"\)/);
  assert.doesNotMatch(source,/\.arg\("native-adapter\.py"\)/);
});
