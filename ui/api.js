// Única puerta entre la interfaz y la app: api.invoke(orden, args) y api.listen(evento, fn).
// Sin Tauri (abriendo el HTML en un navegador) usa un MOCK en memoria con datos de ejemplo;
// ?mock=empty (sin datos) o ?mock=nospaces (sin espacios) en la URL cambian el escenario.
(function () {
  const tauri = window.__TAURI__;
  if (tauri) {
    window.api = {
      invoke: (cmd, args) => tauri.core.invoke(cmd, args),
      listen: (name, fn) => tauri.event.listen(name, (e) => fn(e.payload)),
    };
    return;
  }

  const scenario = new URLSearchParams(location.search).get('mock');
  const MIN = 60e3;
  const HOUR = 36e5;
  const DAY = 864e5;
  const t0 = Date.now();
  let nextId = 100;

  const mk = (o) => ({
    id: nextId++, kind: 'text', preview: '', text: null, imageFile: null, files: [],
    sourceApp: null, favorite: false, spaceId: null, createdAt: t0, usedAt: t0, ...o,
  });
  const at = (ms) => ({ createdAt: t0 - ms, usedAt: t0 - ms });

  let spaces = scenario === 'nospaces' || scenario === 'empty' ? [] : [
    { id: 1, name: 'Trabajo', color: 'orange' },
    { id: 2, name: 'Personal', color: 'blue' },
  ];

  const address = 'Nombre Apellido\nCalle Mayor 12, 3.º B\n28013 Madrid\ncorreo@example.com';
  let items = scenario === 'empty' ? [] : [
    mk({ kind: 'text', text: 'Te paso el enlace de la reunión de mañana a las 10', sourceApp: 'Brave', ...at(2 * MIN) }),
    mk({ kind: 'link', text: 'https://example.com/presupuesto', sourceApp: 'Brave', favorite: true, ...at(14 * MIN) }),
    mk({ kind: 'image', preview: '1920×1080', imageFile: 'captura-factura.png', sourceApp: 'Recortes', ...at(HOUR) }),
    mk({ kind: 'text', text: address, sourceApp: 'Pegatina', favorite: true, spaceId: scenario === 'nospaces' ? null : 1, ...at(2 * HOUR) }),
    mk({ kind: 'files', files: ['C:\\Users\\usuario\\Documents\\contrato-alquiler.pdf'], sourceApp: 'Explorador', ...at(3 * HOUR) }),
    ...(scenario === 'nosecret' ? [] : [
      mk({ kind: 'secret', preview: '••••••••', sourceApp: 'KeePassXC', ...at(5 * HOUR) }),
    ]),
    mk({
      kind: 'text',
      text: 'Hola:\n\nTe confirmo que el jueves a las 17:00 nos vemos en la oficina para revisar el presupuesto de la reforma. '
        + 'Llevaré las facturas del primer trimestre y el contrato firmado. Si necesitas algo más antes, dímelo y lo preparo.\n\nUn abrazo.',
      sourceApp: 'Outlook', ...at(DAY),
    }),
    mk({
      kind: 'files',
      files: ['C:\\Users\\usuario\\Descargas\\factura-septiembre.pdf', 'C:\\Users\\usuario\\Descargas\\recibo-luz.pdf', 'C:\\Users\\usuario\\Descargas\\notas.txt'],
      sourceApp: 'Explorador', ...at(DAY + 2 * HOUR),
    }),
    mk({ kind: 'link', text: 'https://es.wikipedia.org/wiki/Portapapeles', sourceApp: 'Edge', spaceId: scenario === 'nospaces' ? null : 2, ...at(2 * DAY) }),
    mk({ kind: 'text', text: 'Receta de la tortilla: 6 huevos, 4 patatas medianas, 1 cebolla, aceite de oliva y sal.', sourceApp: 'Notas', ...at(4 * DAY) }),
  ];
  for (const it of items) {
    it.preview = it.preview || (it.kind === 'files' ? it.files.map((f) => f.split('\\').pop()).join(', ') : (it.text || '').slice(0, 300));
  }

  let settings = {
    autostart: true, hotkey: 'Ctrl+Alt+Space', textDir: null, mediaDir: null,
    historyLimit: 1000, savePasswords: false, theme: 'system',
  };
  const defaultText = 'C:\\Users\\usuario\\AppData\\Roaming\\Pegatina';
  const withResolved = () => {
    const textDirResolved = settings.textDir || defaultText;
    return { ...settings, textDirResolved, mediaDirResolved: settings.mediaDir || `${textDirResolved}\\multimedia` };
  };

  const listeners = {};
  const emit = (name, payload = null) => (listeners[name] || []).forEach((fn) => fn(payload));
  const changed = () => emit('items-changed');

  function fakeImage() {
    const c = document.createElement('canvas');
    c.width = 480;
    c.height = 270;
    const g = c.getContext('2d');
    const grad = g.createLinearGradient(0, 0, 480, 270);
    grad.addColorStop(0, '#cfe3f7');
    grad.addColorStop(1, '#f7d9c4');
    g.fillStyle = grad;
    g.fillRect(0, 0, 480, 270);
    g.fillStyle = '#fff';
    g.fillRect(40, 30, 400, 210);
    g.fillStyle = '#005fb8';
    g.fillRect(64, 54, 120, 16);
    g.fillStyle = '#d4d4d4';
    for (let y = 96; y < 214; y += 22) g.fillRect(64, y, y % 44 ? 250 : 320, 10);
    return c.toDataURL('image/png');
  }

  function list(f) {
    let r = items.slice();
    if (f.view === 'favorites') r = r.filter((i) => i.favorite);
    else if (f.view === 'space') r = r.filter((i) => i.spaceId === f.spaceId);
    if (f.kind) r = r.filter((i) => i.kind === f.kind);
    if (f.query) {
      const q = f.query.toLowerCase();
      r = r.filter((i) => [i.text, i.preview, ...i.files].some((s) => s && s.toLowerCase().includes(q)));
    }
    return r.sort((a, b) => b.usedAt - a.usedAt).slice(0, f.limit || 200);
  }

  const find = (id) => {
    const it = items.find((i) => i.id === id);
    if (!it) throw 'No se encuentra ese elemento';
    return it;
  };

  const orders = {
    list_items: ({ filter }) => list(filter || {}),
    copy_item: ({ id }) => { find(id).usedAt = Date.now(); console.info('[mock] copiado', id); changed(); return null; },
    paste_item: ({ id }) => { find(id).usedAt = Date.now(); console.info('[mock] pegado', id); changed(); return null; },
    reveal_secret: ({ id }) => { find(id); return 'Zorro-Azul-4471!'; },
    image_data: ({ id }) => { find(id); return fakeImage(); },
    set_favorite: ({ id, value }) => { find(id).favorite = !!value; changed(); return null; },
    set_space: ({ id, spaceId }) => { find(id).spaceId = spaceId ?? null; changed(); return null; },
    delete_item: ({ id }) => { find(id); items = items.filter((i) => i.id !== id); changed(); return null; },
    list_spaces: () => spaces,
    create_space: ({ name, color }) => {
      const s = { id: nextId++, name, color };
      spaces = [...spaces, s];
      return s;
    },
    rename_space: ({ id, name }) => { spaces = spaces.map((s) => (s.id === id ? { ...s, name } : s)); return null; },
    delete_space: ({ id }) => {
      spaces = spaces.filter((s) => s.id !== id);
      items.forEach((i) => { if (i.spaceId === id) i.spaceId = null; });
      changed();
      return null;
    },
    get_settings: () => withResolved(),
    set_settings: ({ settings: s }) => {
      if (['Ctrl+C', 'Ctrl+V', 'Ctrl+Alt+Delete', 'Alt+F4'].includes(s.hotkey)) throw 'Ese atajo ya lo usa otra aplicación';
      const { textDirResolved, mediaDirResolved, ...rest } = s;
      settings = { ...settings, ...rest };
      emit('settings-changed');
      return null;
    },
    pick_folder: () => 'D:\\Pegatina\\Datos',
    move_storage: ({ which, path }) => {
      settings = { ...settings, [which === 'text' ? 'textDir' : 'mediaDir']: path };
      emit('settings-changed');
      return null;
    },
    clear_history: () => { items = items.filter((i) => i.favorite || i.spaceId != null); changed(); return null; },
    open_main: ({ section }) => { window.open(section === 'settings' ? 'main.html#settings' : 'main.html', '_blank'); return null; },
    hide_panel: () => null,
  };

  window.api = {
    isMock: true,
    emit,
    invoke: async (cmd, args = {}) => {
      if (!orders[cmd]) throw `Orden desconocida: ${cmd}`;
      return structuredClone(await orders[cmd](args));
    },
    listen: async (name, fn) => {
      (listeners[name] = listeners[name] || []).push(fn);
      return () => { listeners[name] = listeners[name].filter((f) => f !== fn); };
    },
  };
})();
