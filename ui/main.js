(() => {
  const {
    h, ico, dot, chip, run, ago, fmtDate, basename, dirname, itemTitle, kindName, menu, confirmDialog, toast, errMsg,
    KIND_ICON, SPACE_COLORS, COLOR_NAME,
  } = ui;
  const $ = (id) => document.getElementById(id);
  const q = $('q');

  const KIND_PILLS = [['all', 'Todo'], ['text', 'Texto'], ['link', 'Enlaces'], ['image', 'Imágenes'], ['files', 'Archivos'], ['secret', 'Ocultos']];
  const VIEW_NAME = { recent: 'Recientes', favorites: 'Favoritos' };

  const state = {
    page: 'library', view: 'recent', spaceId: null, kind: 'all', query: '',
    items: [], spaces: [], hasSecrets: false, selectedId: null, copiedId: null, reveal: null,
    renamingId: null, draft: null, settings: null,
  };
  let seq = 0;
  let copiedTimer = 0;
  let listSig = '';
  let detailSig = '';
  let fullImage = { key: '', src: '' };

  const spaceOf = (id) => state.spaces.find((s) => s.id === id);
  const itemOf = (id) => state.items.find((i) => i.id === id);
  const viewName = () => (state.view === 'space' ? (spaceOf(state.spaceId) || { name: 'espacio' }).name : VIEW_NAME[state.view]);

  /* ---------- Datos de la biblioteca ---------- */

  async function refresh() {
    const mine = ++seq;
    try {
      const spaces = await api.invoke('list_spaces');
      if (state.view === 'space' && !spaces.some((s) => s.id === state.spaceId)) { state.view = 'recent'; state.spaceId = null; }
      const filter = { view: state.view };
      if (state.view === 'space') filter.spaceId = state.spaceId;
      if (state.kind !== 'all') filter.kind = state.kind;
      if (state.query.trim()) filter.query = state.query.trim();
      const [items, secrets] = await Promise.all([
        api.invoke('list_items', { filter }),
        api.invoke('list_items', { filter: { view: 'recent', kind: 'secret', limit: 1 } }),
      ]);
      if (mine !== seq) return;
      if (!secrets.length && state.kind === 'secret') { state.kind = 'all'; refresh(); return; }
      const spacesChanged = JSON.stringify(spaces) !== JSON.stringify(state.spaces);
      state.spaces = spaces;
      state.items = items;
      state.hasSecrets = secrets.length > 0;
      if (!itemOf(state.selectedId)) { state.selectedId = items.length ? items[0].id : null; state.reveal = null; }
      if (spacesChanged) renderNav();
      renderChrome();
      renderList();
      renderDetail();
    } catch (e) {
      toast(errMsg(e));
    }
  }

  function setView(view, spaceId) {
    state.view = view;
    state.spaceId = spaceId == null ? null : spaceId;
    state.kind = 'all';
    state.query = '';
    q.value = '';
    state.selectedId = null;
    state.reveal = null;
    setPage('library');
    renderNav();
    return refresh();
  }

  async function copy(id) {
    if (!(await run('copy_item', { id }))) return;
    state.copiedId = id;
    clearTimeout(copiedTimer);
    copiedTimer = setTimeout(() => { state.copiedId = null; renderDetail(); }, 1000);
    renderDetail();
    await refresh();
  }

  async function toggleFavorite(it) {
    if (await run('set_favorite', { id: it.id, value: !it.favorite })) await refresh();
  }

  async function moveTo(id, spaceId) {
    if (await run('set_space', { id, spaceId })) await refresh();
  }

  async function remove(it) {
    if (it.favorite) {
      const ok = await confirmDialog({
        title: '¿Borrar este favorito?', body: 'Se borra del todo y no se puede deshacer.', ok: 'Borrar', danger: true,
      });
      if (!ok) return;
    }
    const at = state.items.indexOf(it);
    const next = state.items[at + 1] || state.items[at - 1];
    if (!(await run('delete_item', { id: it.id }))) return;
    state.selectedId = next ? next.id : null;
    state.reveal = null;
    await refresh();
  }

  /* ---------- Navegación ---------- */

  function renderNav() {
    for (const b of document.querySelectorAll('#nav-library [data-view]')) {
      b.setAttribute('aria-current', String(state.view === b.dataset.view));
    }
    $('spaces-block').hidden = state.spaces.length === 0;
    $('spaces').replaceChildren(...state.spaces.map(spaceRow));
    renderNewSpace();
  }

  function spaceRow(s) {
    if (state.renamingId === s.id) return renameRow(s);
    const current = state.view === 'space' && state.spaceId === s.id;
    const more = h('button', {
      type: 'button', class: 'icon-btn navmore', 'aria-label': `Más acciones de ${s.name}`, 'aria-haspopup': 'menu',
      onclick: () => spaceMenu(s, more),
    }, ico('more'));
    return h('div', {
      class: 'navrow',
      oncontextmenu: (e) => { e.preventDefault(); spaceMenu(s, { x: e.clientX, y: e.clientY }); },
    },
    h('button', { type: 'button', class: 'navbtn', 'aria-current': String(current), onclick: () => setView('space', s.id) },
      h('span', { class: 'dot-box' }, dot(s.color)), h('span', { class: 'name', text: s.name })),
    more);
  }

  function spaceMenu(s, anchor) {
    menu(anchor, [
      { label: 'Renombrar', icon: 'edit', onSelect: () => startRename(s) },
      { label: 'Borrar espacio', icon: 'trash', danger: true, onSelect: () => deleteSpace(s) },
    ]);
  }

  function startRename(s) {
    state.renamingId = s.id;
    renderNav();
    const input = $('spaces').querySelector('.rename');
    if (input) { input.focus(); input.select(); }
  }

  function renameRow(s) {
    let done = false;
    const input = h('input', { class: 'rename', type: 'text', value: s.name, maxlength: '40', 'aria-label': `Nuevo nombre de ${s.name}`, autocomplete: 'off' });
    const finish = async (save) => {
      if (done) return;
      done = true;
      const name = input.value.trim();
      state.renamingId = null;
      if (save && name && name !== s.name) await run('rename_space', { id: s.id, name });
      await refresh();
      renderNav();
    };
    input.addEventListener('keydown', (e) => {
      if (e.key === 'Enter') { e.preventDefault(); finish(true); }
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); finish(false); }
    });
    input.addEventListener('blur', () => finish(true));
    return h('div', { class: 'navrow' }, h('div', { class: 'navbtn' }, h('span', { class: 'dot-box' }, dot(s.color)), input));
  }

  async function deleteSpace(s) {
    const ok = await confirmDialog({
      title: `¿Borrar el espacio «${s.name}»?`,
      body: 'Lo que hay dentro no se borra: solo deja de estar en el espacio.',
      ok: 'Borrar', danger: true,
    });
    if (!ok || !(await run('delete_space', { id: s.id }))) return;
    if (state.view === 'space' && state.spaceId === s.id) { state.view = 'recent'; state.spaceId = null; }
    await refresh();
    renderNav();
  }

  function renderNewSpace() {
    const slot = $('new-space');
    const d = state.draft;
    if (!d) {
      slot.replaceChildren(h('button', { type: 'button', class: 'link-btn', onclick: startNewSpace }, '+ Nuevo espacio'));
      return;
    }
    const create = h('button', { type: 'button', class: 'btn primary', text: 'Crear', disabled: d.name.trim() ? null : true, onclick: submitSpace });
    const input = h('input', { class: 'field', type: 'text', maxlength: '40', placeholder: 'Nombre del espacio', 'aria-label': 'Nombre del espacio', value: d.name, autocomplete: 'off' });
    input.addEventListener('input', () => { d.name = input.value; create.disabled = !input.value.trim(); });
    const swatches = h('div', { class: 'swatches', role: 'radiogroup', 'aria-label': 'Color' }, SPACE_COLORS.map((c) => h('button', {
      type: 'button', role: 'radio', class: 'swatch', 'data-color': c, 'aria-label': COLOR_NAME[c], title: COLOR_NAME[c], 'aria-checked': String(c === d.color),
      onclick: (e) => {
        d.color = c;
        swatches.querySelectorAll('.swatch').forEach((b) => b.setAttribute('aria-checked', String(b.dataset.color === c)));
        if (e.detail > 0) input.focus();
      },
    })));
    const form = h('div', { class: 'space-form', role: 'group', 'aria-label': 'Nuevo espacio' }, input, swatches,
      h('div', { class: 'form-btns' }, create, h('button', { type: 'button', class: 'btn', text: 'Cancelar', onclick: cancelNewSpace })));
    form.addEventListener('keydown', (e) => {
      if (e.key === 'Enter' && e.target === input) { e.preventDefault(); submitSpace(); }
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); cancelNewSpace(); }
    });
    slot.replaceChildren(form);
    input.focus();
  }

  function startNewSpace() {
    const used = new Set(state.spaces.map((s) => s.color));
    state.draft = { name: '', color: SPACE_COLORS.find((c) => !used.has(c)) || SPACE_COLORS[0] };
    renderNewSpace();
  }

  function cancelNewSpace() {
    state.draft = null;
    renderNewSpace();
  }

  async function submitSpace() {
    const d = state.draft;
    const name = d && d.name.trim();
    if (!name) return;
    try {
      const space = await api.invoke('create_space', { name, color: d.color });
      state.draft = null;
      await setView('space', space.id);
    } catch (e) {
      toast(errMsg(e));
    }
  }

  /* ---------- Lista ---------- */

  function renderChrome() {
    const name = viewName();
    q.placeholder = `Buscar en ${name}`;
    q.setAttribute('aria-label', `Buscar en ${name}`);
    const pills = KIND_PILLS.filter(([k]) => k !== 'secret' || state.hasSecrets);
    $('pills').replaceChildren(...pills.map(([k, label]) => h('button', {
      type: 'button', class: 'pill', text: label, 'aria-pressed': String(state.kind === k),
      onclick: () => { state.kind = k; state.selectedId = null; state.reveal = null; refresh(); },
    })));
  }

  const libTitle = (it) => (it.kind === 'image' || it.kind === 'secret' ? it.preview : itemTitle(it));

  function libRow(it) {
    const meta = [kindName(it), ago(it.usedAt), it.sourceApp].filter(Boolean).join(' · ');
    return h('li', { class: 'item item--lib', 'data-id': it.id, 'aria-current': String(it.id === state.selectedId) },
      h('button', { type: 'button', class: 'item-hit', 'data-act': 'select' },
        h('span', { class: 'item-icon' }, ico(KIND_ICON[it.kind], 20)),
        h('span', { class: 'item-main' },
          h('span', { class: 'item-title', text: libTitle(it) }),
          h('span', { class: 'item-meta' }, h('span', { text: meta })))),
      h('button', {
        type: 'button', class: `icon-btn${it.favorite ? ' on' : ''}`, 'data-act': 'star',
        'aria-label': it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos',
        title: it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos',
      }, ico('star')));
  }

  function renderList() {
    const sig = state.items.map((i) => [i.id, i.favorite, i.spaceId, i.usedAt].join()).join('|') + state.selectedId + state.view + state.kind;
    const list = $('list');
    const none = state.items.length === 0;
    list.hidden = none;
    $('empty').hidden = !none;
    if (none) renderEmpty();
    if (sig === listSig) return;
    listSig = sig;
    list.replaceChildren(...state.items.map(libRow));
  }

  function renderEmpty() {
    let icon = 'clipboard';
    let title = 'Todavía no has copiado nada';
    let sub = 'Copia algo y aparecerá aquí.';
    if (state.query.trim() || state.kind !== 'all') {
      icon = 'search'; title = 'No hay nada con eso'; sub = 'Prueba con otra palabra o con otro tipo.';
    } else if (state.view === 'favorites') {
      icon = 'star'; title = 'Aún no tienes favoritos'; sub = 'Marca con la estrella lo que quieras tener a mano.';
    } else if (state.view === 'space') {
      icon = 'folder'; title = 'Este espacio está vacío'; sub = 'Mueve cosas aquí desde el detalle de cada elemento.';
    }
    $('empty').replaceChildren(h('span', { class: 'ico' }, ico(icon, 32)), h('strong', { text: title }), h('span', { text: sub }));
  }

  function select(id) {
    if (id !== state.selectedId) state.reveal = null;
    state.selectedId = id;
    for (const li of $('list').children) li.setAttribute('aria-current', String(Number(li.dataset.id) === id));
    renderDetail();
  }

  /* ---------- Detalle ---------- */

  async function toggleReveal(it) {
    if (state.reveal && state.reveal.id === it.id) {
      state.reveal = null;
    } else {
      try {
        state.reveal = { id: it.id, value: await api.invoke('reveal_secret', { id: it.id }) };
      } catch (e) {
        toast(errMsg(e));
        return;
      }
    }
    renderDetail();
  }

  function imageCard(it) {
    const box = h('div', { class: 'card d-img-box' }, h('span', { class: 'd-loading', text: 'Cargando imagen…' }));
    const key = it.imageFile || `id${it.id}`;
    const show = (src) => { if (box.isConnected) box.replaceChildren(h('img', { class: 'd-img', src, alt: `Imagen copiada, ${it.preview}`, draggable: 'false' })); };
    if (fullImage.key === key) {
      show(fullImage.src);
    } else {
      api.invoke('image_data', { id: it.id })
        .then((src) => { fullImage = { key, src }; show(src); })
        .catch(() => { if (box.isConnected) box.replaceChildren(h('span', { class: 'd-loading', text: 'No se pudo cargar la imagen' })); });
    }
    return box;
  }

  function secretCard(it) {
    const shown = state.reveal && state.reveal.id === it.id;
    return h('div', { class: 'card d-field' },
      h('div', { class: 'd-field-main' },
        h('div', { class: 'd-label', text: 'Contraseña' }),
        h('div', { class: `d-value selectable${shown ? ' mono' : ''}`, text: shown ? state.reveal.value : it.preview || '••••••••' })),
      h('button', {
        type: 'button', class: 'icon-btn lg', 'aria-label': shown ? 'Ocultar contraseña' : 'Mostrar contraseña',
        title: shown ? 'Ocultar' : 'Mostrar', onclick: () => toggleReveal(it),
      }, ico(shown ? 'eyeOff' : 'eye')));
  }

  function filesCard(it) {
    return h('div', { class: 'card' }, it.files.map((f) => h('div', { class: 'frow' },
      h('span', { class: 'ico' }, ico('file', 20)),
      h('div', { class: 'frow-text' },
        h('div', { class: 'frow-name selectable', text: basename(f) }),
        h('div', { class: 'frow-dir selectable', text: dirname(f) })))));
  }

  function spaceCard(it) {
    const select = h('select', { class: 'sel', id: 'sel-space', 'aria-label': 'Espacio' },
      h('option', { value: '', text: 'Sin espacio' }),
      state.spaces.map((s) => h('option', { value: String(s.id), text: s.name })));
    select.value = it.spaceId == null ? '' : String(it.spaceId);
    select.addEventListener('change', () => moveTo(it.id, select.value === '' ? null : Number(select.value)));
    return h('div', { class: 'card d-field' },
      h('div', { class: 'd-field-main' },
        h('label', { class: 'd-value', for: 'sel-space', text: 'Espacio' }),
        h('div', { class: 'd-label', text: 'Lo que está en un espacio no caduca.' })),
      select);
  }

  const headTitle = (it) => (it.kind === 'image' ? `Imagen ${it.preview}` : kindName(it));

  function renderDetail() {
    const box = $('detail');
    const it = itemOf(state.selectedId);
    const copied = it && state.copiedId === it.id;
    const sig = JSON.stringify([it, copied, state.reveal, state.spaces]);
    if (sig === detailSig) return;
    detailSig = sig;
    if (!it) {
      box.replaceChildren(state.items.length
        ? h('div', { class: 'd-none' }, h('span', { class: 'ico' }, ico('clipboard', 32)), h('span', { text: 'Elige un elemento para verlo aquí' }))
        : '');
      return;
    }
    const space = spaceOf(it.spaceId);
    let body;
    if (it.kind === 'image') body = imageCard(it);
    else if (it.kind === 'files') body = filesCard(it);
    else if (it.kind === 'secret') body = secretCard(it);
    else body = h('div', { class: 'card' }, h('pre', { class: 'd-text selectable', text: it.text == null ? it.preview : it.text }));

    const meta = [`Copiado el ${fmtDate(it.createdAt)}`, it.sourceApp ? `desde ${it.sourceApp}` : null, `último uso ${ago(it.usedAt)}`].filter(Boolean).join(' · ');
    box.replaceChildren(...[
      h('div', { class: 'd-head' },
        h('div', { class: 'd-id' },
          h('span', { class: 'd-tile' }, ico(KIND_ICON[it.kind], 20)),
          h('h1', { class: 'd-title', text: headTitle(it) }),
          space ? chip(space) : null),
        h('div', { class: 'd-actions' },
          h('button', { type: 'button', class: 'btn', onclick: () => copy(it.id) }, ico(copied ? 'check' : 'copy'), copied ? 'Copiado' : 'Copiar'),
          h('button', {
            type: 'button', class: `icon-btn lg${it.favorite ? ' on' : ''}`, onclick: () => toggleFavorite(it),
            'aria-label': it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos', title: it.favorite ? 'Quitar de favoritos' : 'Añadir a favoritos',
          }, ico('star')),
          h('button', { type: 'button', class: 'icon-btn lg', 'aria-label': 'Borrar', title: 'Borrar', onclick: () => remove(it) }, ico('trash')))),
      body,
      state.spaces.length ? spaceCard(it) : null,
      h('div', { class: 'd-meta', text: meta }),
    ].filter(Boolean));
  }

  /* ---------- Configuración ---------- */

  const MODS = ['Ctrl', 'Alt', 'Shift', 'Super'];
  const KEY_LABEL = {
    ctrl: 'Ctrl', control: 'Ctrl', commandorcontrol: 'Ctrl', cmdorctrl: 'Ctrl', alt: 'Alt', option: 'Alt', shift: 'Mayús',
    super: 'Win', meta: 'Win', win: 'Win', command: 'Win', cmd: 'Win', space: 'Espacio', enter: 'Intro', escape: 'Esc',
    delete: 'Supr', backspace: 'Retroceso', arrowup: '↑', arrowdown: '↓', arrowleft: '←', arrowright: '→',
    pageup: 'Re Pág', pagedown: 'Av Pág', minus: '-', equal: '=', comma: ',', period: '.', slash: '/', backslash: '\\',
    semicolon: ';', quote: "'", bracketleft: '[', bracketright: ']', backquote: '`',
  };
  const keyLabel = (k) => KEY_LABEL[k.toLowerCase()] || (k.length === 1 ? k.toUpperCase() : k);
  const PLAIN_KEYS = new Set(['Space', 'Enter', 'Tab', 'Backspace', 'Delete', 'Insert', 'Home', 'End', 'PageUp', 'PageDown',
    'ArrowUp', 'ArrowDown', 'ArrowLeft', 'ArrowRight', 'Minus', 'Equal', 'Comma', 'Period', 'Slash', 'Backslash', 'Semicolon',
    'Quote', 'BracketLeft', 'BracketRight', 'Backquote']);
  let capturing = false;

  function keyName(code) {
    const m = /^(?:Key([A-Z])|Digit(\d))$/.exec(code);
    if (m) return m[1] || m[2];
    return /^(F\d{1,2}|Numpad\d)$/.test(code) || PLAIN_KEYS.has(code) ? code : null;
  }

  function renderKeys(hotkey) {
    $('keys').replaceChildren(...hotkey.split('+').map((k) => h('kbd', { class: 'kbd', text: keyLabel(k) })));
  }

  function hotkeyError(msg) {
    $('hotkey-error').hidden = !msg;
    $('hotkey-error').textContent = msg || '';
  }

  function stopCapture() {
    capturing = false;
    document.removeEventListener('keydown', onCaptureKey, true);
    $('btn-hotkey').textContent = 'Cambiar';
    renderKeys(state.settings.hotkey);
  }

  async function onCaptureKey(e) {
    e.preventDefault();
    e.stopPropagation();
    const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
    if (e.key === 'Escape' && !mods.length) { stopCapture(); return; }
    if (['Control', 'Alt', 'Shift', 'Meta', 'AltGraph'].includes(e.key)) return;
    if (!mods.length) { hotkeyError('Usa también Ctrl, Alt, Mayús o la tecla Windows.'); return; }
    const key = keyName(e.code);
    if (!key) { hotkeyError('Esa tecla no se puede usar en un atajo.'); return; }
    stopCapture();
    const err = await save({ hotkey: [...MODS.filter((m) => mods.includes(m)), key].join('+') });
    hotkeyError(err);
  }

  function startCapture() {
    if (capturing) { stopCapture(); hotkeyError(''); return; }
    capturing = true;
    hotkeyError('');
    $('keys').replaceChildren(h('span', { class: 'keys-hint', text: 'Pulsa la nueva combinación…' }));
    $('btn-hotkey').textContent = 'Cancelar';
    document.addEventListener('keydown', onCaptureKey, true);
  }

  function setSwitch(name, on) {
    $(`sw-${name}`).setAttribute('aria-checked', String(on));
    $(`st-${name}`).textContent = on ? 'Activado' : 'Desactivado';
  }

  function renderSettings() {
    const s = state.settings;
    if (!s) return;
    setSwitch('savepw', s.savePasswords);
    setSwitch('autostart', s.autostart);
    if (!capturing) renderKeys(s.hotkey);
    $('path-text').textContent = s.textDirResolved;
    $('path-media').textContent = s.mediaDirResolved;
    const sel = $('sel-history');
    if (![...sel.options].some((o) => o.value === String(s.historyLimit))) {
      sel.append(h('option', { value: String(s.historyLimit), text: `Últimos ${s.historyLimit} elementos` }));
    }
    sel.value = String(s.historyLimit);
    for (const b of document.querySelectorAll('#seg-theme [data-choice]')) b.setAttribute('aria-pressed', String(b.dataset.choice === s.theme));
  }

  async function loadSettings() {
    try {
      state.settings = await api.invoke('get_settings');
      ui.applyTheme(state.settings.theme);
      renderSettings();
    } catch (e) {
      toast(errMsg(e));
    }
  }

  // Devuelve el texto del error si la app rechaza el cambio, o null.
  async function save(patch) {
    const { textDirResolved, mediaDirResolved, ...current } = state.settings;
    try {
      await api.invoke('set_settings', { settings: { ...current, ...patch } });
    } catch (e) {
      renderSettings();
      return errMsg(e);
    }
    state.settings = { ...state.settings, ...patch };
    if (patch.theme) ui.applyTheme(patch.theme);
    renderSettings();
    return null;
  }

  async function saveOrToast(patch) {
    const err = await save(patch);
    if (err) toast(err);
  }

  async function changeFolder(which, btn) {
    let path;
    try {
      path = await api.invoke('pick_folder');
    } catch (e) {
      toast(errMsg(e));
      return;
    }
    if (!path) return;
    const ok = await confirmDialog({ title: '¿Mover tus datos a esta carpeta?', body: 'Se moverá lo que ya tienes guardado.', detail: path, ok: 'Mover' });
    if (!ok) return;
    btn.disabled = true;
    btn.textContent = 'Moviendo…';
    const moved = await run('move_storage', { which, path });
    btn.disabled = false;
    btn.textContent = 'Cambiar';
    await loadSettings();
    if (moved) toast('Listo: tus datos están en la carpeta nueva.');
  }

  async function clearHistory() {
    const ok = await confirmDialog({
      title: '¿Vaciar el historial?',
      body: 'Se borra todo lo copiado, menos los favoritos y lo que está en espacios. No se puede deshacer.',
      ok: 'Vaciar', danger: true,
    });
    if (!ok || !(await run('clear_history'))) return;
    toast('Historial vaciado.');
    await refresh();
  }

  function spySection() {
    const box = $('view-settings');
    let current = 'general';
    for (const name of ['general', 'storage', 'appearance']) {
      if ($(`sec-${name}`).offsetTop <= box.scrollTop + 48) current = name;
    }
    for (const b of document.querySelectorAll('#nav-settings [data-section]')) b.setAttribute('aria-current', String(b.dataset.section === current));
  }

  /* ---------- Páginas y eventos ---------- */

  function setPage(page) {
    state.page = page;
    const lib = page === 'library';
    $('nav-library').hidden = !lib;
    $('view-library').hidden = !lib;
    $('nav-settings').hidden = lib;
    $('view-settings').hidden = lib;
    try { history.replaceState(null, '', `#${page}`); } catch (e) { /* sin historial */ }
    if (!lib) spySection();
  }

  for (const b of document.querySelectorAll('#nav-library [data-view]')) b.addEventListener('click', () => setView(b.dataset.view, null));
  $('go-settings').addEventListener('click', () => { setPage('settings'); loadSettings(); });
  $('go-library').addEventListener('click', () => { setPage('library'); refresh(); });
  window.addEventListener('hashchange', () => {
    const page = location.hash === '#settings' ? 'settings' : 'library';
    if (page === state.page) return;
    setPage(page);
    if (page === 'library') refresh();
    else loadSettings();
  });

  for (const b of document.querySelectorAll('#nav-settings [data-section]')) {
    b.addEventListener('click', () => {
      $(`sec-${b.dataset.section}`).scrollIntoView({ behavior: 'smooth', block: 'start' });
      for (const o of document.querySelectorAll('#nav-settings [data-section]')) o.setAttribute('aria-current', String(o === b));
    });
  }
  $('view-settings').addEventListener('scroll', spySection);

  q.addEventListener('input', () => {
    state.query = q.value;
    state.selectedId = null;
    state.reveal = null;
    refresh();
  });
  $('list').addEventListener('click', (e) => {
    const li = e.target.closest('li.item');
    const btn = e.target.closest('button[data-act]');
    if (!li || !btn) return;
    const id = Number(li.dataset.id);
    if (btn.dataset.act === 'star') toggleFavorite(itemOf(id));
    else select(id);
  });

  $('sw-savepw').addEventListener('click', () => saveOrToast({ savePasswords: !state.settings.savePasswords }));
  $('sw-autostart').addEventListener('click', () => saveOrToast({ autostart: !state.settings.autostart }));
  $('btn-hotkey').addEventListener('click', startCapture);
  $('btn-text-dir').addEventListener('click', (e) => changeFolder('text', e.currentTarget));
  $('btn-media-dir').addEventListener('click', (e) => changeFolder('media', e.currentTarget));
  $('sel-history').addEventListener('change', (e) => saveOrToast({ historyLimit: Number(e.target.value) }));
  $('btn-clear').addEventListener('click', clearHistory);
  for (const b of document.querySelectorAll('#seg-theme [data-choice]')) b.addEventListener('click', () => saveOrToast({ theme: b.dataset.choice }));

  window.addEventListener('blur', () => {
    if (!state.reveal) return;
    state.reveal = null;
    renderDetail();
  });
  api.listen('items-changed', () => { if (state.page === 'library') refresh(); });
  api.listen('settings-changed', loadSettings);

  ui.hydrateIcons();
  renderNav();
  loadSettings();
  if (location.hash === '#settings') setPage('settings');
  else setPage('library');
  if (state.page === 'library') refresh();
})();
