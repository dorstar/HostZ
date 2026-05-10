// ═══ HostZ — Pure JS Frontend ═══
(function() {
'use strict';

function waitForTauri(cb) {
  if (window.__TAURI__?.core?.invoke) { cb(); return; }
  const check = setInterval(() => {
    if (window.__TAURI__?.core?.invoke) { clearInterval(check); cb(); }
  }, 100);
}

waitForTauri(function() {

const invoke = window.__TAURI__.core.invoke;

// ── State ──
let hostsList = [], trashList = [], currentId = null, currentItem = null, isReadonly = true;
let contextTarget = null;
let expandedIds = new Set();
let isDark = false;
let currentLocale = 'zh-CN';

// ── i18n ──
const MSG = {
  'zh-CN': {
    apply:'应用到系统', save:'保存', cancel:'取消', close:'关闭',
    find:'查找替换', settings:'偏好设置',
    quickToggle:'快速切换', addItem:'添加条目', editItem:'编辑条目',
    addHosts:'添加 Hosts', trash:'回收站', systemHosts:'System Hosts',
    toggleSidebar:'切换侧栏',
    readonly:'只读', name:'名称', type:'类型', local:'本地', remote:'远程',
    group:'组合', url:'URL', refreshInterval:'刷新间隔', groupItems:'组合条目',
    noAvailable:'暂无可用条目', never:'从不', minute:'分钟', hour:'小时', day:'天',
    edit:'编辑', copy:'复制', refreshRemote:'刷新远程', deleteTrash:'移入回收站',
    restore:'恢复', deletePerm:'永久删除', clearTrash:'清空回收站',
    confirmDelete:'确定将 "${title}" 移入回收站？',
    confirmPermDelete:'确定永久删除 "${title}"？此操作不可撤销。',
    confirmClearTrash:'确定清空回收站？此操作不可撤销。',
    confirmClearHistory:'确定清空所有历史记录？',
    applySuccess:'已应用到系统 Hosts', applyFailed:'应用失败',
    applyNoEnabled:'没有已启用的条目，请先打开开关',
    saveSuccess:'已保存', saveFailed:'保存失败', refreshSuccess:'刷新成功',
    refreshFailed:'刷新失败', added:'已添加', updated:'已更新', copied:'已复制',
    copyFailed:'复制失败', movedTrash:'已移入回收站', permDeleted:'已永久删除',
    restored:'已恢复', trashCleared:'回收站已清空', importSuccess:'导入成功',
    importFailed:'导入失败', exportSuccess:'导出成功', exportFailed:'导出失败',
    opFailed:'操作失败', settingsSaved:'设置已保存', settingsSaveFailed:'保存设置失败',
    exportTo:'已导出到', errorNeedAdmin:'请以管理员身份运行 HostZ',
    unknownError:'未知错误', copySuffix:' (副本)', findMatches:' 个匹配',
    theme:'主题', light:'浅色', dark:'深色', system:'跟随系统',
    panelWidth:'左侧面板宽度', writeMode:'写入模式',
    append:'追加（推荐）', overwrite:'覆盖', choiceMode:'选择模式',
    multi:'多选', single:'单选', historyLimit:'历史记录上限',
    removeDup:'删除重复记录（待开发）', autoUpdate:'自动检查更新（待开发）',
    genData:'数据', genExport:'导出数据', genImport:'导入数据',
    genHistory:'写入历史', genUpdate:'更新', appearance:'外观',
    hostsWrite:'Hosts 写入', language:'语言', zh:'简体中文', en:'English',
    findQuery:'关键词（支持正则）', findReplace:'替换为…',
    findRegex:'正则', findIC:'忽略大小写', findBtn:'查找',
    findReplaceAll:'替换全部', findReplaced:'已替换', findCount:'个条目',
    paste:'粘贴', selectAll:'全选',
    historyTitle:'Hosts 写入历史', historyEmpty:'暂无记录',
    historyClear:'清空历史', historyAppend:'追加模式', historyOverwrite:'覆盖模式',
    statusLines:'行', statusBytes:'B', statusRO:'只读',
    editorPlaceholder:'选择一个条目查看内容…',
    interval5:'5 分钟', interval15:'15 分钟', interval30:'30 分钟',
    interval1h:'1 小时', interval24h:'24 小时', interval3d:'3 天',
  },
  'en': {
    apply:'Apply to System', save:'Save', cancel:'Cancel', close:'Close',
    find:'Find & Replace', settings:'Preferences',
    quickToggle:'Quick Toggle', addItem:'Add Item', editItem:'Edit Item',
    addHosts:'Add Hosts', trash:'Trash', systemHosts:'System Hosts',
    toggleSidebar:'Toggle Sidebar',
    readonly:'Read-only', name:'Name', type:'Type', local:'Local', remote:'Remote',
    group:'Group', url:'URL', refreshInterval:'Refresh Interval', groupItems:'Group Items',
    noAvailable:'No items available', never:'Never', minute:' m', hour:' h', day:' d',
    edit:'Edit', copy:'Copy', refreshRemote:'Refresh Remote', deleteTrash:'Move to Trash',
    restore:'Restore', deletePerm:'Permanently Delete', clearTrash:'Clear Trash',
    confirmDelete:'Move "${title}" to trash?',
    confirmPermDelete:'Permanently delete "${title}"? This cannot be undone.',
    confirmClearTrash:'Clear all trash? This cannot be undone.',
    confirmClearHistory:'Clear all history?',
    applySuccess:'Applied to system Hosts', applyFailed:'Apply failed',
    applyNoEnabled:'No items enabled. Turn on a switch first.',
    saveSuccess:'Saved', saveFailed:'Save failed', refreshSuccess:'Refreshed',
    refreshFailed:'Refresh failed', added:'Added', updated:'Updated', copied:'Copied',
    copyFailed:'Copy failed', movedTrash:'Moved to trash', permDeleted:'Permanently deleted',
    restored:'Restored', trashCleared:'Trash cleared', importSuccess:'Import successful',
    importFailed:'Import failed', exportSuccess:'Export successful', exportFailed:'Export failed',
    opFailed:'Operation failed', settingsSaved:'Settings saved', settingsSaveFailed:'Failed to save settings',
    exportTo:'Exported to', errorNeedAdmin:'Please run HostZ as administrator',
    unknownError:'Unknown error', copySuffix:' (copy)', findMatches:' matches',
    theme:'Theme', light:'Light', dark:'Dark', system:'System',
    panelWidth:'Left Panel Width', writeMode:'Write Mode',
    append:'Append (recommended)', overwrite:'Overwrite', choiceMode:'Choice Mode',
    multi:'Multiple', single:'Single', historyLimit:'History Limit',
    removeDup:'Remove Duplicates (TBD)', autoUpdate:'Auto Check Updates (TBD)',
    genData:'Data', genExport:'Export Data', genImport:'Import Data',
    genHistory:'Write History', genUpdate:'Update', appearance:'Appearance',
    hostsWrite:'Hosts Write', language:'Language', zh:'Simplified Chinese', en:'English',
    findQuery:'Keywords (regex supported)', findReplace:'Replace with…',
    findRegex:'Regex', findIC:'Ignore Case', findBtn:'Find',
    findReplaceAll:'Replace All', findReplaced:'Replaced', findCount:' items',
    paste:'Paste', selectAll:'Select All',
    historyTitle:'Hosts Write History', historyEmpty:'No records',
    historyClear:'Clear History', historyAppend:'Append', historyOverwrite:'Overwrite',
    statusLines:' lines', statusBytes:' B', statusRO:'Read-only',
    editorPlaceholder:'Select an item to view content…',
    interval5:'5 min', interval15:'15 min', interval30:'30 min',
    interval1h:'1 hour', interval24h:'24 hours', interval3d:'3 days',
  }
};
function tr(key) {
  return (MSG[currentLocale] || MSG['en'])[key] || key;
}
function applyLocale() {
  // Toolbar
  const btnApply = $('btn-apply'); if (btnApply) btnApply.textContent = tr('apply');
  const btnSave = $('btn-save'); if (btnSave) btnSave.textContent = tr('save');
  const btnFindL = $('btn-find-label'); if (btnFindL) btnFindL.textContent = tr('find');
  // Drawer titles
  const lblS = $('label-settings'); if (lblS) lblS.textContent = tr('settings');
  const lblF = $('label-find'); if (lblF) lblF.textContent = tr('find');
  const lblQ = $('label-quick'); if (lblQ) lblQ.textContent = tr('quickToggle');
  // Settings buttons
  const btnSC = $('btn-settings-cancel'); if (btnSC) btnSC.textContent = tr('cancel');
  const btnSS = $('btn-settings-save'); if (btnSS) btnSS.textContent = tr('save');
  // Find panel
  const fq = $('find-query'); if (fq) fq.placeholder = tr('findQuery');
  const fr = $('find-replace'); if (fr) fr.placeholder = tr('findReplace');
  const fgo = $('btn-find-go'); if (fgo) fgo.textContent = tr('findBtn');
  const fra = $('btn-find-replace-all'); if (fra) fra.textContent = tr('findReplaceAll');
  const fRg = $('label-find-regex'); if (fRg) fRg.textContent = tr('findRegex');
  const fIc = $('label-find-ic'); if (fIc) fIc.textContent = tr('findIC');
  // Status bar
  const sRO = $('status-ro'); if (sRO) sRO.textContent = tr('statusRO');
  // Trash
  const thrLbl = $('label-trash'); if (thrLbl) thrLbl.textContent = tr('trash');
  const btnCT = $('btn-clear-trash'); if (btnCT) btnCT.textContent = tr('clearTrash');
  // Topbar readonly badge & editor placeholder
  el.topbarRO.textContent = tr('readonly');
  if (el.editor) el.editor.placeholder = tr('editorPlaceholder');
  // Button tooltips
  const btnTL = $('btn-toggle-left'); if (btnTL) btnTL.title = tr('toggleSidebar');
  const btnAdd = $('btn-add'); if (btnAdd) btnAdd.title = tr('addItem');
  const btnSet = $('btn-settings'); if (btnSet) btnSet.title = tr('settings');
  const btnRef = $('btn-refresh'); if (btnRef) btnRef.title = tr('refreshRemote');
  // SystemHosts in tree
  const shRow = document.querySelector('[data-id="0"] .tree-title');
  if (shRow) shRow.textContent = tr('systemHosts');
  // Update status
  updateStatus();
  // Re-render tree for inline text
  renderTree();
}

// ── IPC Helpers ──
async function ipc(cmd, args) {
  try { return await invoke(cmd, args || {}); }
  catch(e) { console.error('IPC error:', cmd, e); throw e; }
}
async function ipc0(cmd) { return ipc(cmd); }
async function ipc1(cmd, id) { return ipc(cmd, {id}); }
async function ipc2(cmd, id, content) { return ipc(cmd, {id, content}); }

// ── Helpers ──
const $ = id => document.getElementById(id);
const ICON_MAP = {
  'plus-16':'icon-plus','gear-16':'icon-gear','three-bars-16':'icon-bars',
  'sync-16':'icon-sync','search-16':'icon-search','file-16':'icon-file',
  'globe-16':'icon-globe','file-directory-16':'icon-folder','people-16':'icon-people',
  'trash-16':'icon-trash','pencil-16':'icon-pencil','check-16':'icon-check',
  'chevron-right-16':'icon-chevron-right','chevron-down-16':'icon-chevron-down',
  'x-16':'icon-x','copy-16':'icon-copy'
};
function svgIcon(name, cls) {
  const c = ICON_MAP[name] || 'icon-file';
  return `<span class="icon ${c} ${cls||''}"></span>`;
}
function typeIcon(t) {
  const m = {remote:'globe-16',group:'people-16'};
  return svgIcon(m[t]||'file-16');
}
function esc(s) { return String(s||'').replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;'); }
function findItem(items, id) {
  for (const i of items) { if (i.id===id) return i; if (i.children) { const f=findItem(i.children,id); if (f) return f; } }
  return null;
}
function flatList(items, out) {
  out = out || [];
  for (const i of items) { out.push(i); if (i.children) flatList(i.children, out); }
  return out;
}

// ── Toast ──
function toast(msg, type) {
  const t = document.createElement('div');
  t.className = 'toast ' + (type || '');
  t.textContent = msg;
  document.body.appendChild(t);
  requestAnimationFrame(() => { t.classList.add('show'); });
  setTimeout(() => { t.classList.remove('show'); setTimeout(() => t.remove(), 300); }, 2500);
}

// ── Syntax Highlight ──
function highlightHosts(text) {
  return esc(text)
    .replace(/^([ \t]*#.*)$/gm, '<span class="hl-comment">$1</span>')
    .replace(/(\b\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}\b)/g, '<span class="hl-ip">$1</span>')
    .replace(/^(?![ \t]*#)([^#\n]+)/gm, function(m) {
      return m.replace(/(\b[a-zA-Z0-9]([a-zA-Z0-9-]*\.)+[a-zA-Z]{2,}\b)/g, '<span class="hl-host">$1</span>');
    });
}

// ── DOM refs ──
const el = {
  leftPanel: $('left-panel'), hostsTree: $('hosts-tree'), editor: $('editor'),
  editorHL: $('editor-highlight'), editorWrap: $('editor-wrap'),
  topbarIcon: $('topbar-icon'), topbarName: $('topbar-name'), topbarRO: $('topbar-readonly'),
  statusLines: $('status-lines'), statusBytes: $('status-bytes'), statusRO: $('status-ro'), statusSel: $('status-sel'),
  trashSection: $('trash-section'), trashCount: $('trash-count'), trashArrow: $('trash-arrow'), trashList: $('trash-list'),
  drawerSettings: $('drawer-settings'), drawerFind: $('drawer-find'), drawerQuick: $('drawer-quick'),
  quickList: $('quick-list'), findQuery: $('find-query'), findReplace: $('find-replace'),
  findRegex: $('find-regex'), findIC: $('find-ic'), findTotal: $('find-total'), findResults: $('find-results'),
  settingsBody: $('settings-body'),
};

// ── Render Tree ──
function treeRowHTML(item, depth) {
  const indent = depth * 16 + 4;
  const hasKids = item.children && item.children.length > 0;
  const isContainer = item.type === 'group';
  const canExpand = isContainer && hasKids;
  const expanded = expandedIds.has(item.id);

  let html = `<div class="tree-node"><div class="tree-row" data-id="${esc(item.id)}" style="padding-left:${indent}px">`;
  html += `<span class="tree-arrow${canExpand?'':' empty'}"${canExpand?' data-expand="'+esc(item.id)+'"':''}>`;
  html += canExpand ? (expanded ? svgIcon('chevron-down-16','icon-sm') : svgIcon('chevron-right-16','icon-sm')) : '';
  html += `</span>`;
  html += `<span class="tree-icon">${typeIcon(item.type)}</span>`;
  html += `<span class="tree-title">${esc(item.title)}</span>`;
  html += `<span class="tree-actions"><label class="switch${item.on?' on':''}">`;
  html += `<input type="checkbox" data-id="${esc(item.id)}"${item.on?' checked':''}>`;
  html += `<span class="switch-knob"></span></label></span></div>`;
  if (canExpand && expanded && item.children) {
    html += item.children.map(c => treeRowHTML(c, depth + 1)).join('');
  }
  html += '</div>';
  return html;
}

function renderTree() {
  el.hostsTree.innerHTML = hostsList.map(item => treeRowHTML(item, 0)).join('');
  bindTreeEvents();
  renderTrash();
}

function renderTrash() {
  if (!trashList.length) { el.trashSection.style.display='none'; $('btn-clear-trash').style.display='none'; return; }
  el.trashSection.style.display='';
  $('btn-clear-trash').style.display='';
  el.trashCount.textContent = trashList.length;
  el.trashList.innerHTML = trashList.map(t =>
    `<div class="tree-node"><div class="tree-row" style="padding-left:24px" data-id="${esc(t.data.id)}">
      <span class="tree-arrow empty"></span>
      <span class="tree-title trash-item-title">${esc(t.data.title)}</span>
    </div></div>`
  ).join('');
  el.trashList.querySelectorAll('.tree-row').forEach(row => {
    row.addEventListener('contextmenu', e => {
      e.preventDefault();
      const t = trashList.find(x => x.data.id === row.dataset.id);
      if (t) showContextMenu(e, t, true);
    });
  });
}

// ── Tree Events ──
function bindTreeEvents() {
  document.querySelectorAll('#hosts-tree .tree-row').forEach(row => {
    row.addEventListener('click', e => {
      if (e.target.closest('.switch') || e.target.closest('.tree-arrow[data-expand]')) return;
      selectItem(row.dataset.id);
    });
    row.addEventListener('contextmenu', e => {
      e.preventDefault();
      const item = findItem(hostsList, row.dataset.id);
      if (item) showContextMenu(e, item, false);
    });
  });
  document.querySelectorAll('#hosts-tree .switch input').forEach(input => {
    input.addEventListener('change', e => { e.stopPropagation(); toggleItem(input.dataset.id); });
  });
  document.querySelectorAll('#hosts-tree .tree-arrow[data-expand]').forEach(arrow => {
    arrow.addEventListener('click', e => {
      e.stopPropagation();
      const id = arrow.dataset.expand;
      expandedIds.has(id) ? expandedIds.delete(id) : expandedIds.add(id);
      renderTree();
    });
  });
}

// ── Context Menu ──
function ensureContextMenu() {
  if (document.getElementById('ctx-menu')) return;
  const menu = document.createElement('div');
  menu.id = 'ctx-menu'; menu.className = 'context-menu';
  document.body.appendChild(menu);
}

function showContextMenu(e, item, isTrash) {
  ensureContextMenu();
  const menu = document.getElementById('ctx-menu');
  contextTarget = { item, isTrash };
  let items = '';
  if (isTrash) {
    items = `<div class="ctx-item" data-action="restore">${svgIcon('chevron-right-16')} ${tr('restore')}</div>
      <div class="ctx-divider"></div>
      <div class="ctx-item ctx-danger" data-action="perm-delete">${svgIcon('trash-16')} ${tr('deletePerm')}</div>`;
  } else {
    items = `<div class="ctx-item" data-action="edit">${svgIcon('pencil-16')} ${tr('edit')}</div>
      <div class="ctx-item" data-action="copy">${svgIcon('copy-16')} ${tr('copy')}</div>`;
    if (item.type === 'remote') {
      items += `<div class="ctx-item" data-action="refresh">${svgIcon('sync-16')} ${tr('refreshRemote')}</div>`;
    }
    items += `<div class="ctx-divider"></div>
      <div class="ctx-item ctx-danger" data-action="delete">${svgIcon('trash-16')} ${tr('deleteTrash')}</div>`;
  }
  menu.innerHTML = items;
  menu.style.display = 'block';
  menu.style.left = Math.min(e.clientX, window.innerWidth - 160) + 'px';
  menu.style.top = Math.min(e.clientY, window.innerHeight - menu.offsetHeight - 10) + 'px';
  menu.querySelectorAll('.ctx-item').forEach(el => {
    el.addEventListener('click', () => handleContextAction(el.dataset.action));
  });
}

function hideContextMenu() {
  const menu = document.getElementById('ctx-menu');
  if (menu) menu.style.display = 'none';
  contextTarget = null;
}

async function handleContextAction(action) {
  const t = contextTarget;
  hideContextMenu();
  if (!t) return;
  if (action === 'edit') { showAddDialog(t.item); }
  else if (action === 'copy') {
    const src = t.item;
    const newItem = JSON.parse(JSON.stringify(src));
    newItem.id = (crypto.randomUUID ? crypto.randomUUID() : 'xxxx-xxxx-xxxx'.replace(/x/g,()=>'0123456789abcdef'[Math.floor(Math.random()*16)]));
    newItem.title = src.title + tr('copySuffix');
    newItem.on = false;
    const updatedList = [...hostsList, newItem];
    try {
      await ipc('set_list', {listJson: JSON.stringify(updatedList)});
      hostsList = updatedList;
      await loadData();
      toast(tr('copied') + ': ' + newItem.title);
    } catch(e) { toast(tr('copyFailed'), 'error'); }
  }
  else if (action === 'delete') {
    if (!confirm(tr('confirmDelete').replace('${title}', t.item.title))) return;
    try { await ipc1('move_to_trashcan', t.item.id); await loadData(); if (currentId === t.item.id) await selectItem('0'); toast(tr('movedTrash')); }
    catch(e) { toast(tr('opFailed'), 'error'); }
  }
  else if (action === 'refresh') {
    try { await ipc1('refresh_remote', t.item.id); await loadData(); if (currentId === t.item.id) await selectItem(t.item.id); toast(tr('refreshSuccess')); }
    catch(e) { toast(tr('refreshFailed') + ': ' + (e?.toString?.() || e), 'error'); }
  }
  else if (action === 'restore') {
    try { await ipc1('restore_from_trashcan', t.item.data.id); await loadData(); toast(tr('restored')); }
    catch(e) { console.error('Restore failed:', e); }
  }
  else if (action === 'perm-delete') {
    if (!confirm(tr('confirmPermDelete').replace('${title}', t.item.data.title))) return;
    try { await ipc1('permanently_delete', t.item.data.id); await loadData(); toast(tr('permDeleted')); }
    catch(e) { console.error('Permanent delete failed:', e); }
  }
}

// ── Select Item ──
async function selectItem(id) {
  currentId = id;
  document.querySelectorAll('.tree-row').forEach(r => r.classList.remove('selected'));
  const row = document.querySelector(`[data-id="${id}"]`);
  if (row) row.classList.add('selected');

  if (id === '0') {
    el.topbarIcon.innerHTML = ''; el.topbarName.textContent = tr('systemHosts');
    el.topbarRO.style.display = ''; isReadonly = true;
    setEditorMeta(true, false);
    try { const c = await ipc0('get_system_hosts_content'); setEditor(c, true); }
    catch(e) { setEditor('# Error: ' + e, true); }
    return;
  }
  const item = findItem(hostsList, id);
  currentItem = item;
  const ro = item ? item.type !== 'local' : true;
  isReadonly = ro;
  el.topbarIcon.innerHTML = item ? typeIcon(item.type) : '';
  el.topbarName.textContent = item ? item.title : '???';
  el.topbarRO.style.display = ro ? '' : 'none';
  setEditorMeta(ro, item && item.type === 'remote');
  try { const c = await ipc1('get_hosts_content', id); setEditor(c, ro); }
  catch(e) { setEditor('# Error: ' + e, true); }
}

function setEditorMeta(ro, isRemote) {
  $('btn-refresh').style.display = isRemote ? '' : 'none';
  $('btn-save').disabled = ro;
  $('find-replace').disabled = ro;
  $('btn-find-replace-all').disabled = ro;
}

function setEditor(content, ro) {
  el.editor.value = content || '';
  el.editor.classList.toggle('readonly', ro);
  el.statusRO.style.display = ro ? '' : 'none';
  el.editorHL.innerHTML = highlightHosts(content);
  updateStatus();
}

function updateStatus() {
  const c = el.editor.value;
  const lines = c ? c.split('\n').length : 0;
  el.statusLines.textContent = lines + tr('statusLines');
  el.statusBytes.textContent = new Blob([c]).size + tr('statusBytes');
  el.statusSel.textContent = currentId ? (currentId.substring(0,8)+'…') : '';
}

// ── Toggle（由后端 choice_mode 决定单选/多选）──
async function toggleItem(id) {
  try { await ipc1('toggle_hosts', id); await loadData(); }
  catch(e) { console.error('Toggle failed:', e); }
}

// ── Toolbar ──
$('btn-apply').addEventListener('click', async () => {
  let saveOk = true;
  if (currentId && currentId !== '0' && !isReadonly) {
    try { await ipc2('set_hosts_content', currentId, el.editor.value); }
    catch(e) { toast(tr('saveFailed') + ': ' + (e?.toString?.() || e), 'error'); saveOk = false; }
  }
  if (!saveOk) return;

  // 检查是否有条目开关为 ON
  const flat = flatList(hostsList);
  const enabledItems = flat.filter(i => i.on);
  if (!enabledItems.length) {
    toast(tr('applyNoEnabled'), 'error');
    return;
  }

  try { await ipc0('apply_to_system'); toast(tr('applySuccess'), 'success'); }
  catch(e) {
    const msg = (typeof e === 'string') ? e : (e?.message || e?.toString?.() || tr('unknownError'));
    toast(tr('applyFailed') + ': ' + msg, 'error');
  }
});

$('btn-save').addEventListener('click', async () => {
  if (!currentId || currentId === '0' || isReadonly) return;
  try { await ipc2('set_hosts_content', currentId, el.editor.value); toast(tr('saveSuccess')); }
  catch(e) { toast(tr('saveFailed'), 'error'); return; }
  await loadData();
});

$('btn-find').addEventListener('click', () => { el.drawerFind.style.display='flex'; });

$('btn-refresh').addEventListener('click', async () => {
  if (!currentItem || currentItem.type !== 'remote') return;
  const btn = $('btn-refresh');
  const iconEl = btn.querySelector('.icon');
  btn.disabled = true;
  if (iconEl) { iconEl.classList.remove('icon-sync'); iconEl.classList.add('icon-chevron-down'); }
  try { await ipc1('refresh_remote', currentItem.id); await loadData(); await selectItem(currentItem.id); toast(tr('refreshSuccess')); }
  catch(e) { toast(tr('refreshFailed') + ': ' + (e?.toString?.() || e), 'error'); }
  finally {
    if (iconEl) { iconEl.classList.remove('icon-chevron-down'); iconEl.classList.add('icon-sync'); }
    btn.disabled = false;
  }
});

// ── Left Panel Toggle ──
$('btn-toggle-left').addEventListener('click', () => { el.leftPanel.classList.toggle('hidden'); });

// ── Trash toggle ──
$('trash-header').addEventListener('click', () => {
  const list = el.trashList;
  if (list.style.display === 'none') {
    list.style.display = '';
    el.trashArrow.classList.remove('icon-chevron-right');
    el.trashArrow.classList.add('icon-chevron-down');
  } else {
    list.style.display = 'none';
    el.trashArrow.classList.remove('icon-chevron-down');
    el.trashArrow.classList.add('icon-chevron-right');
  }
});

// ── Clear Trash ──
const btnClearTrash = $('btn-clear-trash');
if (btnClearTrash) btnClearTrash.addEventListener('click', async () => {
  if (!trashList.length || !confirm(tr('confirmClearTrash'))) return;
  try { await ipc0('clear_trashcan'); await loadData(); toast(tr('trashCleared')); }
  catch(e) { toast(tr('opFailed'), 'error'); }
});

// ── Import/Export helpers ──
async function doExport() {
  try {
    const dialog = window.__TAURI__?.dialog;
    if (dialog?.save) {
      const path = await dialog.save({
        defaultPath: 'hostz-backup.json',
        filters: [{ name: 'JSON', extensions: ['json'] }]
      });
      if (!path) return; // 用户取消
      await ipc('export_to_file', {path: path});
      toast(tr('exportTo') + ': ' + path);
    } else {
      // 回退：浏览器下载
      const raw = await ipc0('export_data');
      const blob = new Blob([raw], {type:'application/json'});
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a'); a.href = url; a.download = 'hostz-backup.json'; a.click();
      URL.revokeObjectURL(url); toast(tr('exportSuccess'));
    }
  } catch(e) { toast(tr('exportFailed') + ': ' + (e?.toString?.() || e), 'error'); }
}

async function doImport() {
  try {
    const dialog = window.__TAURI__?.dialog;
    if (dialog?.open) {
      const path = await dialog.open({
        filters: [{ name: 'JSON', extensions: ['json'] }],
        multiple: false
      });
      if (!path) return; // 用户取消
      const text = await ipc('read_file', {path: path});
      await ipc('import_data', {jsonStr: text});
      await loadData();
      toast(tr('importSuccess'));
    } else {
      // 回退：浏览器文件选择
      const input = document.createElement('input');
      input.type = 'file'; input.accept = '.json';
      input.addEventListener('change', async () => {
        const file = input.files[0];
        if (!file) return;
        try {
          const text = await file.text();
          await ipc('import_data', {jsonStr: text});
          await loadData();
          toast(tr('importSuccess'));
        } catch(e) { toast(tr('importFailed') + ': ' + (e?.toString?.() || e), 'error'); }
      });
      input.click();
    }
  } catch(e) { toast(tr('importFailed') + ': ' + (e?.toString?.() || e), 'error'); }
}

// ── History ──
async function showHistory() {
  try {
    const raw = await ipc('get_history', {limit: 50});
    const entries = typeof raw === 'string' ? JSON.parse(raw) : raw;
    let html = '<h2>' + tr('historyTitle') + '</h2>';
    if (!entries.length) { html += '<p style="color:var(--text-weak)">' + tr('historyEmpty') + '</p>'; }
    else {
      html += '<div style="display:flex;gap:8px;margin-bottom:8px"><button class="btn-default" id="btn-clear-history">' + tr('historyClear') + '</button></div>';
      html += entries.map((e,i) => {
        const labelMap = { '追加模式': tr('historyAppend'), '覆盖模式': tr('historyOverwrite') };
        return `<div class="history-row" data-idx="${i}">
          <div class="history-meta">${new Date(e.addTimeMs).toLocaleString()} ${labelMap[e.label] || e.label || ''}</div>
          <pre class="history-content">${esc(e.content)}</pre>
        </div>`;
      }).join('');
    }
    el.settingsBody.innerHTML = html;
    el.drawerSettings.style.display = 'flex';
    el.drawerSettings.setAttribute('data-mode', 'history');
    $('btn-settings-save').style.display = 'none';
    $('btn-settings-cancel').textContent = tr('close');

    const btnClear = $('btn-clear-history');
    if (btnClear) btnClear.addEventListener('click', async () => {
      if (!confirm(tr('confirmClearHistory'))) return;
      try { await ipc0('clear_history'); showHistory(); }
      catch(e) { toast(tr('opFailed'), 'error'); }
    });
  } catch(e) { console.error('History load failed:', e); }
}

// ── Add/Edit Dialog ──
const REFRESH_PRESETS = [
  { key: 'never', value: 0 }, { key: 'interval5', value: 300 },
  { key: 'interval15', value: 900 }, { key: 'interval30', value: 1800 },
  { key: 'interval1h', value: 3600 }, { key: 'interval24h', value: 86400 }, { key: 'interval3d', value: 259200 },
];

$('btn-add').addEventListener('click', () => { showAddDialog(); });

function showAddDialog(editItem) {
  const isEdit = !!editItem;
  const title = isEdit ? editItem.title : '';
  const type = isEdit ? editItem.type : 'local';
  const url = isEdit ? (editItem.url||'') : '';
  const interval = isEdit ? (editItem.refreshInterval||0) : 0;
  const intervalOptions = REFRESH_PRESETS.map(p =>
    `<option value="${p.value}" ${interval===p.value?'selected':''}>${tr(p.key)}</option>`
  ).join('');

  // 组合类型可选的条目列表（排除自身和已有组合）
  const availableItems = hostsList.filter(i =>
    i.type !== 'group' && (!isEdit || i.id !== editItem.id)
  );
  const includedIds = (isEdit && editItem.include) ? editItem.include : [];
  const groupCheckboxes = availableItems.map(i =>
    `<label class="check-row"><input type="checkbox" value="${esc(i.id)}" ${includedIds.includes(i.id)?'checked':''}> ${typeIcon(i.type)} ${esc(i.title)}</label>`
  ).join('');

  const checked = (v) => type === v ? ' checked' : '';

  const html = `<div class="modal-overlay" id="modal-add"><div class="modal-box">
    <div class="modal-h">${isEdit ? tr('editItem') : tr('addItem')}</div>
    <div class="modal-b">
      <div class="setting-row"><span>${tr('type')}</span>
        <div class="type-radios">
          <label class="radio-row"><input type="radio" name="add-type" value="local"${checked('local')}> ${tr('local')}</label>
          <label class="radio-row"><input type="radio" name="add-type" value="remote"${checked('remote')}> ${tr('remote')}</label>
          <label class="radio-row"><input type="radio" name="add-type" value="group"${checked('group')}> ${tr('group')}</label>
        </div>
      </div>
      <div class="setting-row"><span>${tr('name')}</span><input id="add-title" value="${esc(title)}" maxlength="50"></div>
      <div id="add-remote-opts" style="display:${type==='remote'?'':'none'}">
        <div class="setting-row"><span>${tr('url')}</span><input id="add-url" value="${esc(url)}" placeholder="https://..."></div>
        <div class="setting-row"><span>${tr('refreshInterval')}</span><select id="add-interval">${intervalOptions}</select></div>
      </div>
      <div id="add-group-opts" style="display:${type==='group'?'':'none'}">
        <div class="setting-row"><span>${tr('groupItems')}</span></div>
        <div class="group-check-list">${groupCheckboxes || '<span style="color:var(--text-weak)">' + tr('noAvailable') + '</span>'}</div>
      </div>
    </div>
    <div class="modal-f">
      <button class="btn-default" id="modal-add-cancel">${tr('cancel')}</button>
      <button class="btn-primary" id="modal-add-save">${isEdit ? tr('save') : tr('addItem')}</button>
    </div>
  </div></div>`;
  document.body.insertAdjacentHTML('beforeend', html);

  // Radio change: toggle remote/group options
  document.querySelectorAll('#modal-add input[name="add-type"]').forEach(radio => {
    radio.addEventListener('change', function() {
      document.getElementById('add-remote-opts').style.display = this.value==='remote' ? '' : 'none';
      document.getElementById('add-group-opts').style.display = this.value==='group' ? '' : 'none';
    });
  });
  document.getElementById('modal-add-cancel').addEventListener('click', () => document.getElementById('modal-add').remove());
  document.getElementById('modal-add').addEventListener('click', function(e) { if (e.target === this) this.remove(); });

  document.getElementById('modal-add-save').addEventListener('click', async () => {
    const t = document.getElementById('add-title').value.trim();
    if (!t) return;
    const itemType = document.querySelector('#modal-add input[name="add-type"]:checked')?.value || 'local';
    const itemUrl = document.getElementById('add-url')?.value || null;
    const itemInterval = parseInt(document.getElementById('add-interval').value) || 0;
    // 收集组合选中的条目 ID
    const groupIncludes = itemType === 'group'
      ? [...document.querySelectorAll('#add-group-opts input:checked')].map(cb => cb.value)
      : null;

    try {
      if (isEdit) {
        const item = findItem(hostsList, editItem.id);
        if (item) {
          item.title = t; item.type = itemType;
          if (itemType === 'remote') {
            item.url = itemUrl || undefined;
            item.refreshInterval = itemInterval || undefined;
            item.include = undefined;
          } else if (itemType === 'group') {
            item.url = undefined; item.refreshInterval = undefined;
            item.include = groupIncludes;
          } else {
            item.url = undefined; item.refreshInterval = undefined;
            item.include = undefined;
          }
          await ipc('set_list', {listJson: JSON.stringify(hostsList)});
          toast(tr('updated') + ': ' + t);
        }
      } else {
        // ADD_GROUP_INCLUDE_COMMENT
        const listJson = itemType === 'group'
          ? (() => {
              const newId = (crypto.randomUUID ? crypto.randomUUID() : 'xxxxxxxx'.replace(/x/g,()=>'0123456789abcdef'[Math.floor(Math.random()*16)]));
              const newItem = { id: newId, title: t, type: itemType, on: false, include: groupIncludes, order: hostsList.length };
              return JSON.stringify([...hostsList, newItem]);
            })()
          : null;
        if (listJson) {
          await ipc('set_list', {listJson: listJson});
        } else {
          await ipc('add_item', {title: t, itemType,
            url: itemType==='remote' ? itemUrl : null,
            refreshInterval: itemType==='remote' ? itemInterval : null});
        }
        toast(tr('added') + ': ' + t, 'success');
      }
    } catch(e) { toast(tr('opFailed') + ': ' + (e?.toString?.() || e), 'error'); }
    document.getElementById('modal-add').remove();
    await loadData();
  });
}

// ── Settings ──
$('btn-settings').addEventListener('click', async () => {
  el.drawerSettings.setAttribute('data-mode', 'settings');
  $('btn-settings-save').style.display = '';
  $('btn-settings-cancel').textContent = tr('cancel');
  $('btn-settings-save').textContent = tr('save');
  try {
    const [raw, version] = await Promise.all([ipc0('config_all'), ipc0('get_version')]);
    const config = typeof raw === 'string' ? JSON.parse(raw) : raw;
    config.version = version;
    el.settingsBody.innerHTML = `
      <h2>${tr('appearance')}</h2>
      <div class="setting-row"><span>${tr('theme')}</span>
        <select id="cfg-theme"><option value="light" ${config.theme==='light'?'selected':''}>${tr('light')}</option><option value="dark" ${config.theme==='dark'?'selected':''}>${tr('dark')}</option><option value="system" ${config.theme==='system'?'selected':''}>${tr('system')}</option></select></div>
      <div class="setting-row"><span>${tr('language')}</span>
        <select id="cfg-locale"><option value="zh-CN" ${(config.locale||'zh-CN')==='zh-CN'?'selected':''}>${tr('zh')}</option><option value="en" ${config.locale==='en'?'selected':''}>${tr('en')}</option></select></div>
      <div class="setting-row"><span>${tr('panelWidth')}</span><input type="number" id="cfg-panel-width" value="${config.leftPanelWidth||260}" min="180" max="500"></div>
      <h2>${tr('hostsWrite')}</h2>
      <div class="setting-row"><span>${tr('writeMode')}</span>
        <select id="cfg-write"><option value="append" ${config.writeMode==='append'?'selected':''}>${tr('append')}</option><option value="overwrite" ${config.writeMode==='overwrite'?'selected':''}>${tr('overwrite')}</option></select></div>
      <div class="setting-row"><span>${tr('choiceMode')}</span>
        <select id="cfg-choice"><option value="2" ${(config.choiceMode||2)==2?'selected':''}>${tr('multi')}</option><option value="1" ${config.choiceMode==1?'selected':''}>${tr('single')}</option></select></div>
      <div class="setting-row"><span>${tr('historyLimit')}</span><input type="number" id="cfg-history" value="${config.historyLimit||50}"></div>
      <div class="setting-row"><span>${tr('removeDup')}</span><input type="checkbox" id="cfg-rm-dup" disabled></div>
      <h2>${tr('genUpdate')}</h2>
      <div class="setting-row"><span>${tr('autoUpdate')}</span><input type="checkbox" id="cfg-update" disabled></div>
      <h2>${tr('genData')}</h2>
      <div class="setting-row" style="gap:8px">
        <button class="btn-default" id="btn-export">${tr('genExport')}</button>
        <button class="btn-default" id="btn-import">${tr('genImport')}</button>
        <button class="btn-default" id="btn-history">${tr('genHistory')}</button>
      </div>
      <h2>关于</h2>
      <div class="setting-row" style="flex-direction:column;align-items:flex-start;gap:4px">
        <span id="about-version" style="font-size:13px;color:var(--text)">HostZ v${config.version||'0.1.0'}</span>
        <a href="https://github.com/你的用户名/hostz" target="_blank" style="font-size:12px;color:var(--primary)">github.com/你的用户名/hostz</a>
      </div>
    `;
    el.drawerSettings.style.display='flex';

    // bind export/import/history buttons (inside settings body)
    const btnExp = document.getElementById('btn-export');
    const btnImp = document.getElementById('btn-import');
    const btnHist = document.getElementById('btn-history');
    if (btnExp) btnExp.addEventListener('click', doExport);
    if (btnImp) btnImp.addEventListener('click', doImport);
    if (btnHist) btnHist.addEventListener('click', () => { el.drawerSettings.style.display='none'; showHistory(); });
  } catch(e) { console.error('Settings load failed:', e); }
});

function collectSettingsPartial() {
  return JSON.stringify({
    theme: document.getElementById('cfg-theme')?.value,
    locale: document.getElementById('cfg-locale')?.value,
    leftPanelWidth: parseInt(document.getElementById('cfg-panel-width')?.value) || 260,
    writeMode: document.getElementById('cfg-write')?.value,
    choiceMode: parseInt(document.getElementById('cfg-choice')?.value) || 2,
    historyLimit: parseInt(document.getElementById('cfg-history')?.value) || 50,
    removeDuplicateRecords: document.getElementById('cfg-rm-dup')?.checked || false,
    autoDownloadUpdate: document.getElementById('cfg-update')?.checked || false,
  });
}

function applySettingsToUI() {
  const theme = document.getElementById('cfg-theme')?.value;
  if (theme) applyTheme(theme);
  const locale = document.getElementById('cfg-locale')?.value;
  if (locale && locale !== currentLocale) {
    currentLocale = locale;
    applyLocale();
  }
  const w = parseInt(document.getElementById('cfg-panel-width')?.value);
  if (w) { el.leftPanel.style.width = w + 'px'; }
}

$('btn-settings-close').addEventListener('click', closeSettings);
$('btn-settings-cancel').addEventListener('click', closeSettings);
function closeSettings() {
  if (el.drawerSettings.getAttribute('data-mode') === 'history') {
    el.drawerSettings.setAttribute('data-mode', 'settings');
    $('btn-settings-save').style.display = '';
    $('btn-settings-cancel').textContent = tr('cancel');
    $('btn-settings-save').textContent = tr('save');
    el.drawerSettings.style.display = 'none';
  } else {
    el.drawerSettings.style.display = 'none';
  }
}

$('btn-settings-save').addEventListener('click', async () => {
  const partial = collectSettingsPartial();
  try { await ipc('config_update', {partialJson: partial}); applySettingsToUI(); toast(tr('settingsSaved')); }
  catch(e) { toast(tr('settingsSaveFailed'), 'error'); }
  el.drawerSettings.style.display = 'none';
});

// ── Dark Theme ──
function applyTheme(theme) {
  const prefersDark = theme === 'dark' || (theme === 'system' && window.matchMedia('(prefers-color-scheme:dark)').matches);
  isDark = prefersDark;
  document.documentElement.setAttribute('data-theme', prefersDark ? 'dark' : 'light');
}

// ── Find ──
$('btn-find-go').addEventListener('click', async () => {
  const q = el.findQuery.value; if (!q) return;
  try {
    const raw = await ipc('find_by', {query: q, isRegexp: el.findRegex.checked, isIgnoreCase: el.findIC.checked});
    const items = typeof raw === 'string' ? JSON.parse(raw) : raw;
    const total = items.reduce((s, r) => s + r.positions.length, 0);
    el.findTotal.textContent = total + tr('findMatches');
    el.findResults.innerHTML = items.flatMap(r =>
      r.positions.map(p => `
        <div class="find-row" data-id="${esc(r.itemId)}">
          <div class="find-match">${esc(p.before)}<mark>${esc(p.match)}</mark>${esc(p.after)}</div>
          <div class="find-title">${typeIcon(r.itemType)} ${esc(r.itemTitle)}</div>
          <div class="find-ln">${p.line}</div>
        </div>`)
    ).join('');
    el.findResults.querySelectorAll('.find-row').forEach(row => {
      row.addEventListener('click', () => { selectItem(row.dataset.id); el.drawerFind.style.display='none'; });
    });
  } catch(e) { console.error('Find failed:', e); }
});

$('btn-find-replace-all').addEventListener('click', async () => {
  const q = el.findQuery.value; if (!q) return;
  try {
    const count = await ipc('find_and_replace_all', {
      query: q, replacement: el.findReplace.value,
      isRegexp: el.findRegex.checked, isIgnoreCase: el.findIC.checked,
    });
    toast(tr('findReplaced') + ' ' + count + ' ' + tr('findCount'));
    el.drawerFind.style.display='none';
  } catch(e) { toast(tr('opFailed'), 'error'); }
});

$('btn-find-close').addEventListener('click', () => { el.drawerFind.style.display='none'; });

// ── Quick Toggle ──
async function showQuickToggle() {
  const flat = flatList(hostsList);
  el.quickList.innerHTML = flat.map(item => `
    <div class="quick-row">
      <span class="qtitle">${esc(item.title)}</span>
      <label class="switch${item.on?' on':''}">
        <input type="checkbox" data-id="${esc(item.id)}" ${item.on?'checked':''}>
        <span class="switch-knob"></span>
      </label>
    </div>`).join('');
  el.quickList.querySelectorAll('input').forEach(inp => {
    inp.addEventListener('change', () => toggleItem(inp.dataset.id));
  });
  el.drawerQuick.style.display='flex';
}
$('btn-quick-close').addEventListener('click', () => { el.drawerQuick.style.display='none'; });

// ── Overlay close ──
[el.drawerSettings, el.drawerFind, el.drawerQuick].forEach(drawer => {
  drawer.addEventListener('click', function(e) { if (e.target === this) this.style.display='none'; });
});

// ── Context menu global hide ──
document.addEventListener('click', hideContextMenu);
document.addEventListener('scroll', hideContextMenu, true);

// ── 右面板自定义编辑右键菜单 ──
$('editor-wrap').addEventListener('contextmenu', e => {
  e.preventDefault();
  showEditorContextMenu(e);
});

function showEditorContextMenu(e) {
  ensureContextMenu();
  const menu = document.getElementById('ctx-menu');
  contextTarget = null;
  const ed = el.editor;
  const isRo = isReadonly;
  const sel = isRo ? window.getSelection().toString() : ed.value.substring(ed.selectionStart, ed.selectionEnd);
  const hasSelection = !!sel;

  menu.innerHTML = `
    <div class="ctx-item" data-action="editor-copy" ${hasSelection?'':'style="opacity:0.4"'}>${svgIcon('copy-16')} ${tr('copy')}</div>
    <div class="ctx-item" data-action="editor-paste" ${isRo?'style="opacity:0.4"':''}>📋 ${tr('paste')}</div>
    <div class="ctx-divider"></div>
    <div class="ctx-item" data-action="editor-select-all">${tr('selectAll')}</div>
  `;
  menu.style.display = 'block';
  menu.style.left = Math.min(e.clientX, window.innerWidth - 160) + 'px';
  menu.style.top = Math.min(e.clientY, window.innerHeight - menu.offsetHeight - 10) + 'px';
  menu.querySelectorAll('.ctx-item').forEach(item => {
    item.addEventListener('click', () => {
      const action = item.dataset.action;
      if (action === 'editor-copy') {
        const text = isRo ? window.getSelection().toString() || el.editorHL.textContent
          : ed.value.substring(ed.selectionStart, ed.selectionEnd) || ed.value;
        navigator.clipboard.writeText(text);
      } else if (action === 'editor-paste') {
        if (isRo) return;
        navigator.clipboard.readText().then(t => {
          const s = ed.selectionStart, e = ed.selectionEnd;
          ed.value = ed.value.substring(0, s) + t + ed.value.substring(e);
          ed.focus();
          ed.selectionStart = ed.selectionEnd = s + t.length;
        });
      } else if (action === 'editor-select-all') {
        if (isRo) {
          const r = document.createRange();
          r.selectNodeContents(el.editorHL);
          window.getSelection().removeAllRanges();
          window.getSelection().addRange(r);
        } else {
          ed.select();
          ed.focus();
        }
      }
      hideContextMenu();
    });
  });
}

// ── Left panel empty area context menu ──
$('left-scroll').addEventListener('contextmenu', e => {
  if (e.target.closest('.tree-row')) return; // let tree-row handler handle it
  e.preventDefault();
  ensureContextMenu();
  const menu = document.getElementById('ctx-menu');
  contextTarget = null;
  menu.innerHTML = `<div class="ctx-item" data-action="add-hosts">${svgIcon('plus-16')} ${esc(tr('addHosts'))}</div>`;
  menu.style.display = 'block';
  menu.style.left = Math.min(e.clientX, window.innerWidth - 160) + 'px';
  menu.style.top = Math.min(e.clientY, window.innerHeight - 40) + 'px';
  menu.querySelector('.ctx-item').addEventListener('click', () => {
    hideContextMenu();
    showAddDialog();
  });
});

// ── Editor input ──
el.editor.addEventListener('input', updateStatus);

// ── Find Enter ──
el.findQuery.addEventListener('keydown', function(e) { if (e.key==='Enter') $('btn-find-go').click(); });

// ── Panel resize ──
(function initResize() {
  const handle = document.createElement('div');
  handle.className = 'resize-handle';
  el.leftPanel.appendChild(handle);
  let dragging = false, startX = 0, startW = 0;
  handle.addEventListener('mousedown', e => {
    dragging = true; startX = e.clientX; startW = el.leftPanel.offsetWidth;
    document.body.style.cursor = 'col-resize'; document.body.style.userSelect = 'none';
  });
  document.addEventListener('mousemove', e => {
    if (!dragging) return;
    const w = Math.max(180, Math.min(500, startW + e.clientX - startX));
    el.leftPanel.style.width = w + 'px';
  });
  document.addEventListener('mouseup', () => {
    if (dragging) { dragging = false; document.body.style.cursor = ''; document.body.style.userSelect = ''; }
  });
})();

// ── Tauri Events ──
try {
  if (window.__TAURI__?.event?.listen) {
    window.__TAURI__.event.listen('show_preferences', () => $('btn-settings').click());
    window.__TAURI__.event.listen('show_quick_toggle', showQuickToggle);
    window.__TAURI__.event.listen('trigger_check_update', () => toast(tr('autoUpdate') + '…'));
    window.__TAURI__.event.listen('reload_list', loadData);
    window.__TAURI__.event.listen('config_theme_changed', (e) => applyTheme(e.payload));
    window.__TAURI__.event.listen('config_updated', () => {});
  }
} catch(e) { console.warn('Tauri event listen failed:', e); }

// ── Init ──
async function loadData() {
  try {
    const raw = await ipc0('get_basic_data');
    const data = typeof raw === 'string' ? JSON.parse(raw) : raw;
    hostsList = data.list || [];
    trashList = data.trashcan || [];
    renderTree();
    console.log('HostZ loaded:', hostsList.length, 'hosts,', trashList.length, 'in trash');
  } catch(e) { console.error('Load data failed:', e); }
}

// ── Initial theme & locale from config ──
async function initTheme() {
  try {
    const raw = await ipc0('config_all');
    const config = typeof raw === 'string' ? JSON.parse(raw) : raw;
    applyTheme(config.theme || 'light');
    if (config.locale) currentLocale = config.locale;
    if (config.leftPanelWidth) el.leftPanel.style.width = config.leftPanelWidth + 'px';
  } catch(e) {}
}

// ── Startup ──
(async function init() {
  document.querySelector('[data-id="0"]').addEventListener('click', () => selectItem('0'));
  await initTheme();
  applyLocale();
  await loadData();
})();

}); // end waitForTauri
})();
