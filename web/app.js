'use strict';
const $ = id => document.getElementById(id);
let nodes = [], selected = null, token = '', busy = false, eventVersion = -1, deleteTarget = null;
let rcHeld = null, rcTimer = null, rcSeq = 0;
let noticeTimer, nodesVersion = '', formDirty = false, renderedTarget = null;
function notice(message, error = false) {
  $('notice').textContent = message;
  $('notice').classList.toggle('error', error);
  $('notice').hidden = false;
  clearTimeout(noticeTimer);
  noticeTimer = setTimeout(() => { $('notice').hidden = true; }, 4500);
}
async function api(path, method = 'GET', body, keepalive = false) {
  const response = await fetch('/api/v1' + path, {
    method, keepalive, headers: method === 'GET' ? {} : {'Content-Type':'application/json','X-NowCtrl-Token':token},
    body: method === 'GET' ? undefined : JSON.stringify(body ?? {}), signal: AbortSignal.timeout(5000)
  });
  const result = await response.json();
  if (!response.ok || !result.ok) throw new Error(result.error || '请求失败');
  return result.data;
}
function element(tag, className, text) {
  const el = document.createElement(tag);
  if (className) el.className = className;
  if (text !== undefined) el.textContent = text;
  return el;
}
function renderNodes() {
  const version = JSON.stringify([nodes, selected]);
  if (version === nodesVersion) return;
  nodesVersion = version;
  const list = $('nodes');
  list.replaceChildren();
  if (!nodes.length) list.append(element('p','empty','还没有节点。开启模拟配对，添加第一个虚拟节点。'));
  nodes.forEach((node, index) => {
    const row = element('div','node-row' + (selected === node.id ? ' selected' : ''));
    const choose = element('button','node-select');
    choose.type = 'button';
    choose.setAttribute('aria-pressed', String(selected === node.id));
    choose.setAttribute('aria-label', `选择 CH${node.channel} ${node.name}`);
    choose.append(element('span','channel-box',`CH${String(node.channel).padStart(2,'0')}`));
    const info = element('span');
    const title = element('span','node-title',node.name);
    title.append(element('span','node-type',profileName(node.device_type)));
    info.append(title);
    const meta = element('span','node-meta');
    meta.append(element('span','dot ' + (node.online ? 'online-dot' : 'offline-dot')));
    meta.append(document.createTextNode(`${node.online ? '模拟在线' : '已停用'} · ID ${node.id.toString(16).toUpperCase()}`));
    info.append(meta); choose.append(info);
    choose.addEventListener('click', () => { stopRc(); selected = node.id; renderNodes(); renderTarget(true); });
    row.append(choose);
    const order = element('span','order-buttons');
    for (const [direction, label] of [[-1,'上移'],[1,'下移']]) {
      const move = element('button','',direction < 0 ? '↑' : '↓');
      move.type = 'button'; move.title = label;
      move.setAttribute('aria-label',`${label}${node.name}`);
      move.disabled = busy || index + direction < 0 || index + direction >= nodes.length;
      move.addEventListener('click', () => mutate(async () => {
        const ids = nodes.map(n => n.id);
        [ids[index], ids[index + direction]] = [ids[index + direction], ids[index]];
        await api('/nodes/order','PUT',{node_ids:ids});
        notice('通道顺序已保存，节点 ID 保持不变');
      }));
      order.append(move);
    }
    row.append(order); list.append(row);
  });
  $('node-count').replaceChildren(document.createTextNode(String(nodes.length).padStart(2,'0')+' '),element('small','','/ 08'));
  $('online-count').textContent = String(nodes.filter(n => n.online).length).padStart(2,'0');
}
function renderTarget(resetForm = false) {
  const node = nodes.find(n => n.id === selected);
  if (resetForm || renderedTarget !== selected) formDirty = false;
  renderedTarget = selected;
  $('control-empty').hidden = !!node;
  $('control-body').hidden = !node;
  $('settings-form').querySelectorAll('input,button').forEach(el => { el.disabled = busy || !node; });
  if (!node) { $('node-name').value = ''; return; }
  $('target-channel').textContent = `CH ${String(node.channel).padStart(2,'0')}`;
  $('target-name').textContent = node.name;
  $('target-id').textContent = `NODE ${node.id.toString(16).toUpperCase().padStart(8,'0')} / VIRTUAL`;
  $('target-online').textContent = node.online ? '模拟在线' : '已停用';
  $('target-online').classList.toggle('offline',!node.online);
  renderPreset(node);
  $('output-value').textContent = node.output ? 'ON' : 'OFF';
  for (const id of ['output-value','output-light','signal-fill']) $(id).classList.toggle('on',node.output);
  document.querySelectorAll('[data-action],#pulse-form button,#pulse-duration').forEach(el => { el.disabled = busy || !node.enabled || node.device_type !== 'switch'; });
  if (!formDirty) $('node-name').value = node.name;
  if (!formDirty) $('node-enabled').checked = node.enabled;
}
function renderEvents(events) {
  const version = events.at(-1)?.id ?? 0;
  if (version === eventVersion) return;
  eventVersion = version;
  const terminal = $('events');
  const atBottom = terminal.scrollTop + terminal.clientHeight >= terminal.scrollHeight - 20;
  terminal.replaceChildren();
  for (const event of events) {
    const row = element('p','event '+event.kind);
    row.append(element('span','event-time',event.time));
    row.append(element('span','event-type',`[${event.kind.toUpperCase()}] `));
    row.append(document.createTextNode(event.message));
    terminal.append(row);
  }
  if (atBottom) terminal.scrollTop = terminal.scrollHeight;
}
async function refresh() {
  const status = await api('/status');
  if (status.mode !== 'simulation' || status.firmware_connected !== false) throw new Error('此页面当前只支持软件模拟服务');
  token = status.mutation_token;
  const [newNodes, events] = await Promise.all([api('/nodes'),api('/demo/events')]);
  nodes = newNodes;
  for (const node of nodes) rcSeq = Math.max(rcSeq, node.rc?.seq ?? 0);
  if (!nodes.some(n => n.id === selected)) selected = nodes[0]?.id ?? null;
  renderNodes(); renderTarget(); renderEvents(events);
  $('connection').textContent = '本机模拟服务已连接';
  $('connection').classList.remove('error');
  $('pairing').hidden = status.pairing_remaining_s <= 0;
  $('pair-time').textContent = `${status.pairing_remaining_s}s`;
  $('pair').disabled = busy || nodes.length >= status.max_nodes;
  $('add-form').querySelectorAll('input,select,button').forEach(el => { el.disabled = busy || nodes.length >= status.max_nodes; });
  const t = status.uptime_s;
  $('uptime').textContent = [Math.floor(t/3600),Math.floor(t/60)%60,t%60].map(n => String(n).padStart(2,'0')).join(':');
}
async function mutate(action) {
  if (busy) return;
  stopRc();
  busy = true;
  document.querySelectorAll('button').forEach(button => { button.disabled = true; });
  try { await action(); }
  catch (error) { notice(error.message, true); }
  finally {
    busy = false;
    nodesVersion = '';
    document.querySelectorAll('button').forEach(button => { button.disabled = false; });
    await refreshSafely();
  }
}
async function refreshSafely() {
  try { await refresh(); }
  catch (error) {
    $('connection').textContent = '服务断开 · 正在重试';
    document.querySelectorAll('button:not(#refresh)').forEach(button => { button.disabled = true; });
    notice(`无法连接本机模拟服务：${error.message}`, true);
  }
}
$('refresh').addEventListener('click',refreshSafely);
$('pair').addEventListener('click',() => mutate(async () => { await api('/pairing/start','POST'); notice('60 秒模拟配对窗口已开启'); }));
$('add-form').addEventListener('submit',event => {
  event.preventDefault();
  mutate(async () => {
    const node = await api('/demo/nodes','POST',{name:$('add-name').value,device_type:$('add-type').value});
    selected = node.id; $('add-name').value = ''; notice('模拟节点已添加');
  });
});
for (const button of document.querySelectorAll('[data-action]')) button.addEventListener('click',() => control(button.dataset.action));
function control(action) {
  const target = selected;
  if (target === null || nodes.find(n => n.id === target)?.device_type !== 'switch') return;
  mutate(async () => { await api(`/nodes/${target}/control`,'POST',{action,duration_ms:Number($('pulse-duration').value)}); notice(`${action} 已应用到虚拟节点`); });
}
$('pulse-form').addEventListener('submit',event => { event.preventDefault(); control('PULSE'); });
$('settings-form').addEventListener('input', () => { formDirty = true; });
$('settings-form').addEventListener('submit',event => {
  event.preventDefault();
  const target = selected;
  mutate(async () => { await api(`/nodes/${target}`,'PATCH',{name:$('node-name').value,enabled:$('node-enabled').checked}); formDirty = false; notice('节点设置已保存'); });
});
$('delete').addEventListener('click',() => {
  const node = nodes.find(n => n.id === selected);
  if (!node) return;
  deleteTarget = node.id;
  $('delete-description').textContent = `CH ${node.channel} · ${node.name} · ID ${node.id}`;
  $('delete-dialog').returnValue = 'cancel';
  $('delete-dialog').showModal();
});
$('delete-dialog').addEventListener('close',() => {
  const target = deleteTarget;
  deleteTarget = null;
  if ($('delete-dialog').returnValue === 'confirm' && target !== null) mutate(async () => { await api(`/nodes/${target}`,'DELETE',{confirm_id:target}); notice('模拟节点已移除'); });
});
async function poll() {
  if (!busy && !rcHeld && !$('delete-dialog').open && !['node-name','node-enabled','add-name','add-type','pulse-duration'].includes(document.activeElement?.id)) await refreshSafely();
  setTimeout(poll,1500);
}

function profileName(type) {
  return ({switch:'开关',rc_basic:'模玩',unknown:'GPIO 只读'})[type] ?? 'GPIO 只读';
}
const rcLabels = {STOP:'停止',FORWARD:'前进',BOOST_FORWARD:'加速前进',REVERSE:'后退',LEFT:'左 · 仅转向',RIGHT:'右 · 仅转向'};
function renderRc(state = {}) {
  $('rc-action').textContent = rcLabels[state.action] ?? '停止';
  $('rc-throttle').textContent = `${state.throttle ?? 0}%`;
  $('rc-steering').textContent = `${state.steering ?? 0}%`;
}
function renderPreset(node) {
  const isSwitch = node.device_type === 'switch', isRc = node.device_type === 'rc_basic';
  $('device-mode').textContent = isSwitch ? 'SWITCH PRESET' : isRc ? 'RC BASIC' : 'GPIO / READ ONLY';
  $('switch-controls').hidden = !isSwitch;
  $('rc-controls').hidden = !isRc;
  $('gpio-controls').hidden = isSwitch || isRc;
  document.querySelectorAll('[data-rc],#rc-stop').forEach(button => { button.disabled = busy || !node.enabled || !isRc; });
  renderRc(node.rc);
  const gpio = $('gpio-values');
  gpio.replaceChildren();
  if (!isSwitch && !isRc) {
    if (!node.gpio?.length) gpio.append(element('p','empty','尚未收到 GPIO 电平上报'));
    for (const pin of node.gpio ?? []) {
      const row = element('div','gpio-row');
      row.append(element('span','',`GPIO ${pin.pin}`));
      row.append(element('span','gpio-level '+(pin.level === true ? 'high' : pin.level === false ? 'low' : ''),
        pin.level === true ? 'HIGH / 高' : pin.level === false ? 'LOW / 低' : '未知 / 未上报'));
      gpio.append(row);
    }
  }
}
function stopRc(force = false) {
  const held = rcHeld;
  rcHeld = null;
  clearTimeout(rcTimer);
  document.querySelectorAll('[data-rc]').forEach(button => button.classList.remove('holding'));
  const id = held?.id ?? (force && nodes.find(n => n.id === selected)?.device_type === 'rc_basic' ? selected : null);
  if (id === null) return;
  const seq = ++rcSeq;
  api(`/nodes/${id}/rc`,'POST',{action:'STOP',seq},true)
    .then(data => { if (selected === id && !rcHeld) renderRc(data); })
    .catch(() => { notice('停止请求未确认；模拟看门狗将在断流后归零',true); });
}
function startRc(button) {
  const node = nodes.find(n => n.id === selected);
  if (busy || !node?.enabled || node.device_type !== 'rc_basic' || rcHeld) return;
  const held = {id:node.id,action:button.dataset.rc};
  rcHeld = held;
  button.classList.add('holding');
  async function send() {
    if (rcHeld !== held) return;
    try {
      const data = await api(`/nodes/${held.id}/rc`,'POST',{action:held.action,seq:++rcSeq});
      if (rcHeld !== held) return;
      renderRc(data);
      rcTimer = setTimeout(send,100);
    } catch (error) {
      if (rcHeld === held) { stopRc(); notice(`模玩更新中断：${error.message}`,true); }
    }
  }
  send();
}
for (const button of document.querySelectorAll('[data-rc]')) {
  button.addEventListener('pointerdown',event => {
    if (!event.isPrimary || event.button !== 0) return;
    event.preventDefault(); button.setPointerCapture(event.pointerId); startRc(button);
  });
  for (const eventName of ['pointerup','pointercancel','lostpointercapture']) button.addEventListener(eventName,() => stopRc());
  button.addEventListener('keydown',event => {
    if ((event.key === ' ' || event.key === 'Enter') && !event.repeat) { event.preventDefault(); startRc(button); }
  });
  button.addEventListener('keyup',event => {
    if (event.key === ' ' || event.key === 'Enter') { event.preventDefault(); stopRc(); }
  });
  button.addEventListener('blur',() => stopRc());
  // Assistive-technology clicks produce a bounded tap, never a latched movement.
  button.addEventListener('click',event => {
    if (event.detail === 0 && !rcHeld) { startRc(button); const held = rcHeld; setTimeout(() => { if (held && rcHeld === held) stopRc(); },150); }
  });
}
$('rc-stop').addEventListener('click',() => stopRc(true));
window.addEventListener('blur',() => stopRc());
window.addEventListener('pagehide',() => stopRc());
document.addEventListener('visibilitychange',() => { if (document.hidden) stopRc(); });

refreshSafely().then(() => setTimeout(poll,1500));
