import {refresh} from './source.mjs';
const host = 'world.herdr.notion_allowance';
let running = false;
async function update() {
  if (running) return;
  running = true;
  try {
    const available = await refresh(value => chrome.runtime.sendNativeMessage(host, value));
    await chrome.action.setBadgeText({text: available ? '' : '!'});
    await chrome.action.setTitle({title: available ? 'Notion allowance refreshed' : 'Notion allowance unavailable. Check the configured browser login.'});
  } catch {
    await chrome.action.setBadgeText({text: '!'});
    await chrome.action.setTitle({title: 'Anton Notion bridge needs setup'});
  } finally { running = false; }
}
async function start() {
  await chrome.alarms.create('notion-allowance', {periodInMinutes: 5});
  await update();
}
chrome.runtime.onInstalled.addListener(start);
chrome.runtime.onStartup.addListener(start);
chrome.alarms.onAlarm.addListener(alarm => {if (alarm.name === 'notion-allowance') void update();});
chrome.action.onClicked.addListener(update);
