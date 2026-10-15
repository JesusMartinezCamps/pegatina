(() => {
  const { h, ico, chip, run, ago, itemTitle, thumbImg, menu, closeMenu, confirmDialog, toast, errMsg, KIND_ICON } = ui;
  const $ = (id) => document.getElementById(id);
  const q = $('q');
  const list = $('list');
  const empty = $('empty');
  const spaceBtn = $('space-btn');

  const state = { tab: 'recent', spaceId: null, query: '', items: [], spaces: [], selectedId: null, copiedId: null };
  let seq = 0;
  let copiedTimer = 0;

  const spaceOf = (id) => state.spaces.find((s) => s.id === id);
  const itemOf = (id) => state.items.find((i) => i.id === id);

  function filter() {
    const f = { view: state.tab === 'favorites' ? 'favorites' : 'recent' };
    if (state.spaceId != null) { f.view = 'space'; f.spaceId = state.spaceId; }
    if (state.query.trim()) f.query = state.query.trim();
    return f;
  }

  async function load() {
    const mine = ++seq;
    try {
      const spaces = await api.invoke('list_spaces');
      if (state.spaceId != null && !spaces.some((s) => s.id === state.spaceId)) state.spaceId = null;
      let items = await api.invoke('list_items', { filter: filter() });
      if (state.tab === 'favorites' && state.spaceId != null) items = items.filter((i) => i.favorite);
      if (mine !== seq) return;
      state.spaces = spaces;
      state.items = items;
      if (!itemOf(state.selectedId)) state.selectedId = items.length ? items[0].id : null;
      render();
    } catch (e) {
      toast(errMsg(e));
    }
  }

  function row(it) {
    const copied = state.copiedId === it.id;
    const space = spaceOf(it.spaceId);
    const title = itemTitle(it);
    const head = it.kind === 'image'
      ? h('div', { class: 'item-img' }, h('div', { class: 'thumb' }, thumbImg(it)), h('div', { class: 'item-title', text: title }))
      : h('div', { class: `item-title${it.kind === 'text' ? ' clamp' : ''}`, text: title });
    const meta = [ago(it.usedAt), it.sourceApp].filter(Boolean).join(' · ');
    return h('li', { class: 'item', 'data-id': it.id, 'aria-current': it.id === state.selectedId ? 'true' : null },
      h('span', { class: 'item-icon' }, ico(KIND_ICON[it.kind], 20)),
      h('div', { class: 'item-main' }, head, h('div', { class: 'item-meta' }, h('span', { text: meta }), space ? chip(space) : null)),
      h('div', { class: 'item-actions' },
        h('button', {
          type: 'button', class: `icon-btn${copied ? ' copied' : ''}`, 'data-act': 'copy',
          'aria-label': copied ? 'Copiado' : 'Copiar', title: 'Copiar',
        }, copied ? [ico('check'), 'Copiado'] : ico('copy')),
        h('button', {
          type: 'button', class: `icon-btn${it.favorite ? ' on' : ''}`, 'data-act': 'star',
          'aria-label': it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos',
          title: it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos',
        }, ico('star')),
        h('button', { type: 'button', class: 'icon-btn', 'data-act': 'more', 'aria-label': 'Más acciones', 'aria-haspopup': 'menu', title: 'Más acciones' }, ico('more'))));
  }

  function renderEmpty() {
    let icon = 'clipboard';
    let title = 'Copia algo y aparecerá aquí';
    let sub = 'Todo se guarda solo, en este ordenador.';
    if (state.query.trim()) {
      icon = 'search'; title = 'No hay nada con eso'; sub = 'Prueba con otra palabra.';
    } else if (state.spaceId != null) {
      icon = 'folder'; title = 'Este espacio está vacío'; sub = 'Mueve cosas aquí desde el menú «más» de cada elemento.';
    } else if (state.tab === 'favorites') {
      icon = 'star'; title = 'Aún no tienes favoritos'; sub = 'Marca con la estrella lo que quieras tener a mano.';
    }
    empty.replaceChildren(h('span', { class: 'ico' }, ico(icon, 32)), h('strong', { text: title }), h('span', { text: sub }));
  }

  function render() {
    for (const b of document.querySelectorAll('[data-tab]')) b.setAttribute('aria-pressed', String(b.dataset.tab === state.tab));

    const sp = spaceOf(state.spaceId);
    spaceBtn.hidden = state.spaces.length === 0;
    const label = sp ? sp.name : 'Todos los espacios';
    $('space-label').textContent = label;
    $('space-dot').hidden = !sp;
    if (sp) $('space-dot').dataset.color = sp.color;
    spaceBtn.setAttribute('aria-label', `Filtrar por espacio: ${label}`);

    list.replaceChildren(...state.items.map(row));
    list.hidden = state.items.length === 0;
    empty.hidden = state.items.length !== 0;
    if (state.items.length === 0) renderEmpty();
  }

  function select(id, scroll) {
    state.selectedId = id;
    for (const li of list.children) {
      const on = Number(li.dataset.id) === id;
      if (on) li.setAttribute('aria-current', 'true');
      else li.removeAttribute('aria-current');
      if (on && scroll) li.scrollIntoView({ block: 'nearest' });
    }
  }

  function move(step) {
    if (!state.items.length) return;
    const i = state.items.findIndex((it) => it.id === state.selectedId);
    const next = Math.min(state.items.length - 1, Math.max(0, i + step));
    select(state.items[next].id, true);
  }

  async function copy(id) {
    if (!(await run('copy_item', { id }))) return;
    state.copiedId = id;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => { state.copiedId = null; render(); }, 1000);
    await load();
  }

  const paste = (id) => run('paste_item', { id });

  async function toggleFavorite(it) {
    if (await run('set_favorite', { id: it.id, value: !it.favorite })) await load();
  }

  async function moveTo(id, spaceId) {
    if (await run('set_space', { id, spaceId })) await load();
  }

  async function remove(id) {
    const it = itemOf(id);
    if (!it) return;
    if (it.favorite) {
      const ok = await confirmDialog({
        title: '¿Borrar este favorito?', body: 'Se borra del todo y no se puede deshacer.', ok: 'Borrar', danger: true,
      });
      q.focus();
      if (!ok) return;
    }
    const at = state.items.indexOf(it);
    const next = state.items[at + 1] || state.items[at - 1];
    if (!(await run('delete_item', { id }))) return;
    state.selectedId = next ? next.id : null;
    await load();
  }

  function openMore(it, anchor) {
    const entries = [];
    if (state.spaces.length) {
      entries.push({
        label: 'Mover a espacio…',
        icon: 'folder',
        submenu: state.spaces.map((s) => ({ label: s.name, dot: s.color, checked: it.spaceId === s.id, onSelect: () => moveTo(it.id, s.id) })),
      });
      if (it.spaceId != null) entries.push({ label: 'Quitar del espacio', icon: 'x', onSelect: () => moveTo(it.id, null) });
      entries.push({ sep: true });
    }
    entries.push({ label: 'Borrar', icon: 'trash', danger: true, onSelect: () => remove(it.id) });
    menu(anchor, entries, { restoreFocus: q });
  }

  list.addEventListener('click', (e) => {
    const li = e.target.closest('li.item');
    if (!li) return;
    const id = Number(li.dataset.id);
    select(id);
    const btn = e.target.closest('button[data-act]');
    const it = itemOf(id);
    if (!btn || !it) return;
    if (btn.dataset.act === 'copy') copy(id);
    else if (btn.dataset.act === 'star') toggleFavorite(it);
    else openMore(it, btn);
  });
  list.addEventListener('dblclick', (e) => {
    const li = e.target.closest('li.item');
    if (li && !e.target.closest('button')) paste(Number(li.dataset.id));
  });

  q.addEventListener('input', () => {
    state.query = q.value;
    state.selectedId = null;
    load();
  });

  for (const b of document.querySelectorAll('[data-tab]')) {
    b.addEventListener('click', () => {
      state.tab = b.dataset.tab;
      state.selectedId = null;
      load();
      q.focus();
    });
  }

  spaceBtn.addEventListener('click', () => {
    const setSpace = (id) => { state.spaceId = id; state.selectedId = null; load(); };
    menu(spaceBtn, [
      { label: 'Todos los espacios', checked: state.spaceId == null, onSelect: () => setSpace(null) },
      { sep: true },
      ...state.spaces.map((s) => ({ label: s.name, dot: s.color, checked: state.spaceId === s.id, onSelect: () => setSpace(s.id) })),
    ], { restoreFocus: q });
  });

  $('btn-library').addEventListener('click', () => run('open_main', { section: 'library' }));
  $('btn-settings').addEventListener('click', () => run('open_main', { section: 'settings' }));

  document.addEventListener('keydown', (e) => {
    if (document.querySelector('dialog[open]')) return;
    const onControl = e.target.closest && e.target.closest('button, select');
    const caretAtEnd = q.selectionStart === q.value.length && q.selectionEnd === q.value.length;
    switch (e.key) {
      case 'Escape':
        run('hide_panel');
        return;
      case 'ArrowDown':
      case 'ArrowUp':
        e.preventDefault();
        move(e.key === 'ArrowDown' ? 1 : -1);
        return;
      case 'Enter':
        if (!onControl && state.selectedId != null) { e.preventDefault(); paste(state.selectedId); }
        return;
      case 'Delete':
        if (e.target === q && !caretAtEnd) return;
        if (state.selectedId != null) { e.preventDefault(); remove(state.selectedId); }
        return;
      case 'c':
      case 'C':
        if (e.ctrlKey && !e.shiftKey && !e.altKey) {
          if (e.target === q && q.selectionStart !== q.selectionEnd) return;
          if (state.selectedId != null) { e.preventDefault(); copy(state.selectedId); }
        }
        return;
      default:
        if (e.key.length === 1 && !e.ctrlKey && !e.metaKey && !e.altKey && e.target !== q && !onControl) q.focus();
    }
  });

  async function loadTheme() {
    try { ui.applyTheme((await api.invoke('get_settings')).theme); } catch (e) { /* tema del sistema */ }
  }

  api.listen('panel-shown', async () => {
    closeMenu();
    document.querySelectorAll('dialog[open]').forEach((d) => d.close());
    q.value = '';
    state.query = '';
    state.selectedId = null;
    state.copiedId = null;
    await load();
    document.querySelector('.p-list').scrollTop = 0;
    q.focus();
  });
  api.listen('items-changed', load);
  api.listen('settings-changed', loadTheme);
  window.addEventListener('focus', () => {
    if (!document.querySelector('.menu, dialog[open]')) q.focus();
  });

  ui.hydrateIcons();
  loadTheme();
  load().then(() => q.focus());
})();
