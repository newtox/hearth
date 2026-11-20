const { app, BrowserWindow, desktopCapturer, ipcMain, net, protocol, session, shell } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');

protocol.registerSchemesAsPrivileged([{ scheme: 'app', privileges: { standard: true, secure: true, supportFetchAPI: true, corsEnabled: true, stream: true } }]);
app.setName('Hearth');
app.commandLine.appendSwitch('enable-features', 'WebRTCPipeWireCapturer');
app.commandLine.appendSwitch('ozone-platform-hint', 'auto');

const root = path.join(__dirname, 'web');
const stateFile = () => path.join(app.getPath('userData'), 'window.json');
const allowed = new Set(['media', 'notifications', 'display-capture', 'fullscreen', 'clipboard-sanitized-write', 'speaker-selection']);
let win = null;
let patchBay = null;

function loadPatchBay() {
  if (patchBay !== null) return patchBay;
  try {
    const { PatchBay } = require('@vencord/venmic');
    patchBay = PatchBay.hasPipeWire() ? new PatchBay() : false;
  } catch {
    patchBay = false;
  }
  return patchBay;
}

function audioServicePid() {
  const service = app.getAppMetrics().find((m) => m.type === 'Utility' && /audio/i.test(`${m.name ?? ''} ${m.serviceName ?? ''}`));
  return service ? String(service.pid) : String(process.pid);
}

function readState() {
  try {
    return JSON.parse(fs.readFileSync(stateFile(), 'utf8'));
  } catch {
    return {};
  }
}

function saveState() {
  if (!win || win.isDestroyed()) return;
  const state = { bounds: win.getNormalBounds(), maximized: win.isMaximized() };
  try {
    fs.writeFileSync(stateFile(), JSON.stringify(state));
  } catch {}
}

function pick(sources) {
  return new Promise((resolve) => {
    const picker = new BrowserWindow({
      parent: win,
      modal: true,
      width: 760,
      height: 520,
      resizable: false,
      autoHideMenuBar: true,
      title: 'Hearth',
      backgroundColor: '#1b1b1f',
      webPreferences: { contextIsolation: true, sandbox: true },
    });
    let done = false;
    const finish = (value) => {
      if (done) return;
      done = true;
      resolve(value);
      if (!picker.isDestroyed()) picker.close();
    };
    picker.webContents.on('did-navigate-in-page', (_e, url) => {
      const id = decodeURIComponent(new URL(url).hash.slice(1));
      finish(sources.find((s) => s.id === id) ?? null);
    });
    picker.on('closed', () => finish(null));
    const items = sources.map((s) => ({ id: s.id, name: s.name, thumb: s.thumbnail.toDataURL() }));
    const url = pathToFileURL(path.join(__dirname, 'picker.html')).toString();
    picker.loadURL(`${url}?data=${encodeURIComponent(JSON.stringify({ lang: app.getLocale(), items }))}`);
  });
}

function createWindow() {
  const state = readState();
  win = new BrowserWindow({
    width: state.bounds?.width ?? 1280,
    height: state.bounds?.height ?? 800,
    x: state.bounds?.x,
    y: state.bounds?.y,
    minWidth: 380,
    minHeight: 560,
    frame: false,
    show: false,
    title: 'Hearth',
    backgroundColor: '#1b1b1f',
    icon: path.join(root, 'icon-512.png'),
    webPreferences: { preload: path.join(__dirname, 'preload.cjs'), contextIsolation: true, sandbox: true, spellcheck: true },
  });
  if (state.maximized) win.maximize();
  win.once('ready-to-show', () => win.show());
  win.on('close', saveState);
  for (const ev of ['maximize', 'unmaximize']) win.on(ev, () => win.webContents.send('window-state', win.isMaximized()));
  win.webContents.setWindowOpenHandler(({ url }) => {
    if (/^https?:/i.test(url)) shell.openExternal(url);
    return { action: 'deny' };
  });
  win.webContents.on('will-navigate', (e, url) => {
    if (!url.startsWith('app://')) {
      e.preventDefault();
      if (/^https?:/i.test(url)) shell.openExternal(url);
    }
  });
  win.loadURL('app://hearth/');
}

app.whenReady().then(() => {
  protocol.handle('app', (request) => {
    const { pathname } = new URL(request.url);
    let file = path.normalize(path.join(root, decodeURIComponent(pathname)));
    if (!file.startsWith(root) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) file = path.join(root, 'index.html');
    return net.fetch(pathToFileURL(file).toString());
  });
  session.defaultSession.setPermissionRequestHandler((_wc, permission, callback) => callback(allowed.has(permission)));
  session.defaultSession.setPermissionCheckHandler((_wc, permission) => allowed.has(permission));
  session.defaultSession.setDisplayMediaRequestHandler(async (_request, callback) => {
    let answered = false;
    const reply = (value) => {
      if (answered) return;
      answered = true;
      try {
        callback(value);
      } catch {}
    };
    try {
      const sources = await desktopCapturer.getSources({ types: ['screen', 'window'], thumbnailSize: { width: 320, height: 180 } });
      const chosen = sources.length === 1 ? sources[0] : sources.length ? await pick(sources) : null;
      reply(chosen ? { video: chosen } : {});
    } catch {
      reply({});
    }
  });
  ipcMain.handle('window', (_e, action) => {
    if (!win) return false;
    if (action === 'minimize') win.minimize();
    if (action === 'toggle-maximize') win.isMaximized() ? win.unmaximize() : win.maximize();
    if (action === 'close') win.close();
    return win.isMaximized();
  });
  ipcMain.handle('open-external', (_e, url) => {
    if (typeof url === 'string' && /^https?:\/\//i.test(url)) shell.openExternal(url);
  });
  ipcMain.handle('audio-share', (_e, on) => {
    const bay = loadPatchBay();
    if (!bay) return false;
    if (!on) {
      try {
        bay.unlink();
      } catch {}
      return true;
    }
    try {
      return bay.link({
        exclude: [{ 'application.process.id': audioServicePid() }, { 'media.class': 'Stream/Input/Audio' }, { 'media.class': 'Audio/Sink' }],
        ignore_devices: true,
        only_speakers: false,
        only_default_speakers: false,
      });
    } catch {
      return false;
    }
  });
  ipcMain.handle('audio-share-available', () => !!loadPatchBay());
  createWindow();
});

app.on('second-instance', () => {
  if (!win) return;
  if (win.isMinimized()) win.restore();
  win.focus();
});

if (!app.requestSingleInstanceLock()) app.quit();

app.on('window-all-closed', () => {
  if (patchBay) patchBay.unlink();
  app.quit();
});
