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
test('the Claude Code mod payload is embedded in the runtime, not installed as plugin files',()=>{
  const mod='hooks/claude/anton-observatory';
  const source=fs.readFileSync('omarchy/anton-runtime/src/hooks_install/claude_mod.rs','utf8');
  for(const file of ['.claude-plugin/plugin.json','hooks/hooks.json','hooks/register.js']){
    assert.ok(fs.statSync(`${mod}/${file}`).isFile(),file);
    assert.ok(source.includes(`include_str!("../../../../${mod}/${file}")`),`embedded: ${file}`);
  }
  const manifest=JSON.parse(fs.readFileSync(`${mod}/.claude-plugin/plugin.json`,'utf8'));
  const version=fs.readFileSync('omarchy/anton-runtime/Cargo.toml','utf8').match(/^version = "(.*)"$/m)[1];
  assert.deepEqual(Object.keys(manifest).sort(),['defaultEnabled','description','name','version']);
  assert.equal(manifest.name,'anton-observatory');
  assert.ok(!manifest.name.startsWith('claude-'));
  assert.equal(manifest.version,version);
  assert.equal(manifest.defaultEnabled,true);
  assert.deepEqual(JSON.parse(fs.readFileSync(`${mod}/hooks/hooks.json`,'utf8')),{modules:['./register.js']});
  assert.equal(fs.readFileSync(`${mod}/hooks/register.js`,'utf8').split("const nativeRuntime = '';").length,2);
  const walk=(dir)=>fs.readdirSync(dir,{withFileTypes:true}).flatMap(e=>e.isDirectory()?walk(`${dir}/${e.name}`):[`${dir}/${e.name}`]);
  for(const path of walk('hooks'))assert.ok(!path.endsWith('.py'),path);
  // The installed plugin file lists are unchanged: the mod lives only in the runtime.
  const installer=fs.readFileSync(`${root}/install.sh`,'utf8');
  const uninstaller=fs.readFileSync(`${root}/uninstall.sh`,'utf8');
  assert.equal(installer.match(/^files=\((.*)\)$/m)[1],'manifest.json Panel.qml PopupContent.qml SectionHeader.qml AntonText.qml AntonSurface.qml ThreadSignal.qml SheenTitle.qml BurnEffect.qml MetricDial.qml ThreadCard.qml AllowanceCard.qml AntonTheme.qml AntonToolTip.qml AntonPreferences.qml AntonController.qml AntonKeyedModel.qml SnapshotStore.qml State.js README.md uninstall.sh');
  assert.equal(uninstaller.match(/^\s*(\.hooks-receipt\.json\|.*)\) ;;$/m)[1],'.hooks-receipt.json|.hooks-before-native.json|.peers.json|anton-runtime|.config.json|manifest.json|Panel.qml|PopupContent.qml|SectionHeader.qml|AntonText.qml|AntonSurface.qml|ThreadSignal.qml|SheenTitle.qml|BurnEffect.qml|MetricDial.qml|ThreadCard.qml|AllowanceCard.qml|AntonTheme.qml|AntonToolTip.qml|AntonPreferences.qml|AntonController.qml|AntonKeyedModel.qml|.accounts.json|SnapshotStore.qml|State.js|README.md|uninstall.sh|.herdr-observatory-install');
  for(const source of [installer,uninstaller])assert.doesNotMatch(source,/anton-observatory|\.claude\//);
});
test('install.sh installs the Claude Code mod after the hooks and only warns on refusal',()=>{
  const installer=fs.readFileSync(`${root}/install.sh`,'utf8');
  const hooks=installer.indexOf('"$target/anton-runtime" --install-hooks\n');
  const mod=installer.indexOf('"$target/anton-runtime" --install-claude-mod || echo "Claude Code context reporter not installed; the context dial stays unknown" >&2\n');
  assert.ok(hooks>=0,'--install-hooks');
  assert.ok(mod>hooks,'--install-claude-mod runs after --install-hooks, with a warning');
  assert.equal(installer.match(/--install-claude-mod/g).length,1);
  assert.doesNotMatch(fs.readFileSync(`${root}/uninstall.sh`,'utf8'),/--uninstall-claude-mod/);
});
