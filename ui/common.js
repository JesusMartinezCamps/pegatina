// Piezas compartidas por el panel y la ventana principal: iconos, DOM, tiempos, menú, diálogo, tema.
// Todo lo que viene del portapapeles entra al DOM solo con textContent (h({text}) o nodos de texto).
(function () {
  const root = document.documentElement;
  const THEME_KEY = 'pegatina-theme';

  function applyTheme(theme) {
    if (theme === 'light' || theme === 'dark') root.dataset.theme = theme;
    else delete root.dataset.theme;
    try { localStorage.setItem(THEME_KEY, theme || 'system'); } catch (e) { /* sin almacenamiento */ }
  }
  try { applyTheme(localStorage.getItem(THEME_KEY)); } catch (e) { /* primer arranque */ }
  if (!window.__TAURI__) root.classList.add('preview');

  const ICONS = {
    search: '<circle cx="7" cy="7" r="4.5"/><path d="M10.5 10.5L14 14"/>',
    clock: '<circle cx="8" cy="8" r="6"/><path d="M8 4.5V8l2.5 1.5"/>',
    star: '<path d="M8 1.8l1.9 3.9 4.3.6-3.1 3 .7 4.3L8 11.6l-3.8 2 .7-4.3-3.1-3 4.3-.6z"/>',
    copy: '<rect x="5.5" y="5.5" width="8" height="8" rx="1.5"/><path d="M10.5 5.5V4A1.5 1.5 0 0 0 9 2.5H4A1.5 1.5 0 0 0 2.5 4v5A1.5 1.5 0 0 0 4 10.5h1.5"/>',
    check: '<path d="M3 8.5l3.2 3.2L13 4.8"/>',
    more: '<circle cx="3" cy="8" r="1.1" fill="currentColor" stroke="none"/><circle cx="8" cy="8" r="1.1" fill="currentColor" stroke="none"/><circle cx="13" cy="8" r="1.1" fill="currentColor" stroke="none"/>',
    text: '<path d="M3.5 4V3h9v1M8 3v10M6 13h4"/>',
    link: '<path d="M6.5 9.5a2.5 2.5 0 0 0 3.5 0l2.5-2.5a2.5 2.5 0 0 0-3.5-3.5l-.8.8"/><path d="M9.5 6.5a2.5 2.5 0 0 0-3.5 0L3.5 9a2.5 2.5 0 0 0 3.5 3.5l.8-.8"/>',
    image: '<rect x="2" y="3" width="12" height="10" rx="1.5"/><circle cx="6" cy="7" r="1"/><path d="M2.5 12l3.5-3.5 2.5 2.5 2-2 3 3"/>',
    file: '<path d="M4 1.8h5l3.5 3.5v8.4a.8.8 0 0 1-.8.8H4a.8.8 0 0 1-.8-.8V2.6a.8.8 0 0 1 .8-.8z"/><path d="M9 1.8v3.5h3.5"/>',
    lock: '<rect x="3.5" y="7" width="9" height="6.5" rx="1.5"/><path d="M5.5 7V5a2.5 2.5 0 0 1 5 0v2"/>',
    grid: '<rect x="2.5" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="9" y="2.5" width="4.5" height="4.5" rx="1"/><rect x="2.5" y="9" width="4.5" height="4.5" rx="1"/><rect x="9" y="9" width="4.5" height="4.5" rx="1"/>',
    gear: '<g transform="scale(.6667)" stroke-width="2.25"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></g>',
    chevron: '<path d="M4 6l4 4 4-4"/>',
    plus: '<path d="M8 3v10M3 8h10"/>',
    eye: '<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/>',
    eyeOff: '<path d="M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/><path d="M2.5 13.5l11-11"/>',
    trash: '<path d="M2.5 4h11M6 4V2.5h4V4M4 4l.7 9.5h6.6L12 4M6.5 7v4M9.5 7v4"/>',
    folder: '<path d="M2 4.5A1.5 1.5 0 0 1 3.5 3h2.8l1.5 1.8h4.7A1.5 1.5 0 0 1 14 6.3v5.7a1.5 1.5 0 0 1-1.5 1.5h-9A1.5 1.5 0 0 1 2 12z"/>',
    sliders: '<path d="M2.5 5h6M11.5 5h2M2.5 11h2M7.5 11h6"/><circle cx="10" cy="5" r="1.5"/><circle cx="6" cy="11" r="1.5"/>',
    sun: '<circle cx="8" cy="8" r="2.2"/><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4"/>',
    back: '<path d="M13 8H3M7 4L3 8l4 4"/>',
    edit: '<path d="M10.5 2.5l3 3L5.5 13.5l-3.5.5.5-3.5z"/><path d="M9 4l3 3"/>',
    x: '<path d="M4 4l8 8M12 4l-8 8"/>',
    clipboard: '<rect x="3.5" y="3" width="9" height="11" rx="1.5"/><path d="M6 3V2.5h4V3"/>',
  };

  function ico(name, size = 16) {
    const t = document.createElement('template');
    t.innerHTML = `<svg width="${size}" height="${size}" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">${ICONS[name] || ''}</svg>`;
    return t.content.firstChild;
  }

  function hydrateIcons(scope = document) {
    scope.querySelectorAll('[data-icon]').forEach((el) => {
      el.replaceChildren(ico(el.dataset.icon, Number(el.dataset.size) || 16));
      delete el.dataset.icon;
    });
  }

  function h(tag, props, ...kids) {
    const el = document.createElement(tag);
    for (const [k, v] of Object.entries(props || {})) {
      if (v == null || v === false) continue;
      if (k === 'class') el.className = v;
      else if (k === 'text') el.textContent = v;
      else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else el.setAttribute(k, v === true ? '' : v);
    }
    for (const kid of kids.flat()) if (kid != null && kid !== false) el.append(kid);
    return el;
  }

  const dot = (color) => h('span', { class: 'dot', 'data-color': color || 'blue' });
  const chip = (space) => h('span', { class: 'chip' }, dot(space.color), h('span', { text: space.name }));

  const KIND_LABEL = { text: 'Texto', link: 'Enlace', image: 'Imagen', files: 'Archivos', secret: 'Contraseña' };
  const KIND_ICON = { text: 'text', link: 'link', image: 'image', files: 'file', secret: 'lock' };
  const SPACE_COLORS = ['blue', 'orange', 'green', 'purple', 'pink', 'teal', 'red', 'gray'];
  const COLOR_NAME = { blue: 'Azul', orange: 'Naranja', green: 'Verde', purple: 'Morado', pink: 'Rosa', teal: 'Turquesa', red: 'Rojo', gray: 'Gris' };

  const dateFmt = new Intl.DateTimeFormat('es-ES', { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  const dayFmt = new Intl.DateTimeFormat('es-ES', { day: 'numeric', month: 'short', year: 'numeric' });
  const fmtDate = (ms) => dateFmt.format(ms);

  function ago(ms) {
    const s = Math.max(0, (Date.now() - ms) / 1000);
    if (s < 45) return 'ahora mismo';
    const m = Math.max(1, Math.floor(s / 60));
    if (m < 60) return `hace ${m} min`;
    const hr = Math.floor(m / 60);
    if (hr < 24) return `hace ${hr} h`;
    const d = Math.floor(hr / 24);
    if (d === 1) return 'ayer';
    if (d < 30) return `hace ${d} días`;
    return dayFmt.format(ms);
  }

  const basename = (p) => p.split(/[\\/]/).filter(Boolean).pop() || p;
  const dirname = (p) => p.slice(0, Math.max(0, p.length - basename(p).length)).replace(/[\\/]$/, '');
  const errMsg = (e) => (typeof e === 'string' ? e : (e && e.message) || String(e));
  const oneLine = (s) => (s || '').replace(/\s+/g, ' ').trim();

  function itemTitle(it) {
    if (it.kind === 'image') return `Imagen ${it.preview}`.trim();
    if (it.kind === 'secret') return `${it.preview || '••••••••'} (contraseña)`;
    if (it.kind === 'files') return it.preview || it.files.map(basename).join(', ');
    return oneLine(it.preview || it.text);
  }

  function kindName(it) {
    if (it.kind === 'files') return it.files.length === 1 ? 'Archivo' : `${it.files.length} archivos`;
    return KIND_LABEL[it.kind] || '';
  }

  // Las miniaturas se pintan solo cuando la fila entra en pantalla y se guardan reducidas (clave: PNG por hash).
  const thumbs = new Map();
  const seen = new IntersectionObserver((entries) => {
    for (const e of entries) {
      if (e.isIntersecting || !e.target.isConnected) seen.unobserve(e.target);
      if (e.isIntersecting) e.target._load();
    }
  });

  function thumbKey(it) { return it.imageFile || `id${it.id}`; }

  async function makeThumb(it) {
    const url = await api.invoke('image_data', { id: it.id });
    const img = await new Promise((res, rej) => {
      const i = new Image();
      i.onload = () => res(i);
      i.onerror = rej;
      i.src = url;
    });
    const k = Math.min(1, 96 / img.width, 64 / img.height);
    const c = document.createElement('canvas');
    c.width = Math.max(1, Math.round(img.width * k));
    c.height = Math.max(1, Math.round(img.height * k));
    c.getContext('2d').drawImage(img, 0, 0, c.width, c.height);
    return c.toDataURL('image/png');
  }

  function thumbImg(it) {
    const img = h('img', { class: 'thumb-img', alt: '', draggable: 'false' });
    const key = thumbKey(it);
    const show = (src) => { if (src) img.src = src; };
    img._load = () => {
      if (!thumbs.has(key)) thumbs.set(key, makeThumb(it).catch(() => null));
      thumbs.get(key).then(show);
    };
    seen.observe(img);
    return img;
  }

  let openMenu = null;
  function closeMenu() { if (openMenu) openMenu.close(false); }

  // items: {label, icon | dot, danger, checked, sep, header, submenu: [...], keepOpen, onSelect}
  function menu(anchor, items, opts = {}) {
    if (openMenu && openMenu.anchor === anchor) { closeMenu(); return null; }
    closeMenu();
    const el = h('div', { class: 'menu', role: 'menu' });
    const back = opts.restoreFocus || (anchor instanceof Element ? anchor : null);
    let closed = false;

    function place() {
      const r = anchor instanceof Element ? anchor.getBoundingClientRect() : { left: anchor.x, right: anchor.x, top: anchor.y, bottom: anchor.y };
      const w = el.offsetWidth;
      const ht = el.offsetHeight;
      let x = anchor instanceof Element ? r.right - w : r.left;
      let y = r.bottom + 4;
      if (y + ht > innerHeight - 8) y = Math.max(8, (anchor instanceof Element ? r.top - 4 : r.top) - ht);
      x = Math.min(Math.max(8, x), innerWidth - w - 8);
      el.style.left = `${x}px`;
      el.style.top = `${y}px`;
    }

    function render(list) {
      el.replaceChildren(...list.map((it) => {
        if (it.sep) return h('div', { class: 'sep', role: 'separator' });
        if (it.header) return h('div', { class: 'menu-h', text: it.header });
        return h('button', {
          type: 'button',
          role: 'menuitem',
          class: it.danger ? 'danger' : '',
          onclick: () => {
            if (it.submenu) {
              render([{ label: 'Atrás', icon: 'back', keepOpen: true, onSelect: () => render(items) }, { sep: true }, ...it.submenu]);
              return;
            }
            if (!it.keepOpen) close(true);
            if (it.onSelect) it.onSelect();
          },
        },
        h('span', { class: 'menu-i' }, it.dot ? dot(it.dot) : it.icon ? ico(it.icon) : null),
        h('span', { class: 'menu-l', text: it.label }),
        it.checked ? h('span', { class: 'menu-i' }, ico('check')) : null);
      }));
      place();
      const first = el.querySelector('button');
      if (first) first.focus();
    }

    function onDown(e) {
      if (el.contains(e.target) || (anchor instanceof Element && anchor.contains(e.target))) return;
      close(false);
    }
    function onKey(e) {
      if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); close(true); return; }
      if (e.key === 'Tab') { close(false); return; }
      if (e.key !== 'ArrowDown' && e.key !== 'ArrowUp') return;
      e.preventDefault();
      e.stopPropagation();
      const btns = [...el.querySelectorAll('button')];
      const i = btns.indexOf(document.activeElement);
      const next = e.key === 'ArrowDown' ? (i + 1) % btns.length : (i - 1 + btns.length) % btns.length;
      btns[next].focus();
    }
    function close(refocus) {
      if (closed) return;
      closed = true;
      document.removeEventListener('pointerdown', onDown, true);
      document.removeEventListener('keydown', onKey, true);
      window.removeEventListener('blur', onBlur);
      el.remove();
      openMenu = null;
      if (refocus && back && back.isConnected) back.focus();
    }
    const onBlur = () => close(false);

    document.body.append(el);
    render(items);
    document.addEventListener('pointerdown', onDown, true);
    document.addEventListener('keydown', onKey, true);
    window.addEventListener('blur', onBlur);
    openMenu = { anchor, close };
    return openMenu;
  }

  function confirmDialog({ title, body, detail, ok = 'Aceptar', cancel = 'Cancelar', danger = false }) {
    closeMenu();
    return new Promise((resolve) => {
      let result = false;
      const okBtn = h('button', { type: 'button', class: 'btn primary', text: ok, autofocus: danger ? null : true });
      const noBtn = h('button', { type: 'button', class: 'btn', text: cancel, autofocus: danger ? true : null });
      const dlg = h('dialog', { class: 'dlg', 'aria-labelledby': 'dlg-title' },
        h('div', { class: 'dlg-body' },
          h('h2', { class: 'dlg-title', id: 'dlg-title', text: title }),
          body ? h('p', { class: 'dlg-text', text: body }) : null,
          detail ? h('p', { class: 'dlg-detail', text: detail }) : null),
        h('div', { class: 'dlg-foot' }, okBtn, noBtn));
      okBtn.addEventListener('click', () => { result = true; dlg.close(); });
      noBtn.addEventListener('click', () => dlg.close());
      dlg.addEventListener('close', () => { dlg.remove(); resolve(result); });
      document.body.append(dlg);
      dlg.showModal();
    });
  }

  async function run(order, args) {
    try {
      await api.invoke(order, args);
      return true;
    } catch (e) {
      toast(errMsg(e));
      return false;
    }
  }

  let toastTimer = 0;
  function toast(message) {
    let el = document.getElementById('toast');
    if (!el) {
      el = h('div', { id: 'toast', class: 'toast', role: 'status' });
      document.body.append(el);
    }
    el.textContent = message;
    el.classList.add('show');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => el.classList.remove('show'), 5000);
  }

  window.ui = {
    applyTheme, ico, hydrateIcons, h, dot, chip, run, ago, fmtDate, basename, dirname, errMsg, oneLine,
    itemTitle, kindName, thumbImg, menu, closeMenu, confirmDialog, toast,
    KIND_ICON, KIND_LABEL, SPACE_COLORS, COLOR_NAME,
  };
})();
