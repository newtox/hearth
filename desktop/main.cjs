const { app, BrowserWindow, desktopCapturer, ipcMain, net, protocol, session, shell } = require('electron');
const fs = require('node:fs');
const path = require('node:path');
const { pathToFileURL } = require('node:url');
const { spawn } = require('node:child_process');

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

let updater = null;

function loadUpdater() {
  if (updater !== null) return updater;
  updater = false;
  if (!app.isPackaged || process.env.FLATPAK_ID) return updater;
  try {
    const { autoUpdater } = require('electron-updater');
    autoUpdater.autoDownload = false;
    autoUpdater.autoInstallOnAppQuit = false;
    autoUpdater.logger = null;
    autoUpdater.on('download-progress', (p) => win?.webContents.send('update-progress', p.percent));
    updater = autoUpdater;
  } catch {}
  return updater;
}

const flatpak = !!process.env.FLATPAK_ID;
const repo = (() => {
  const target = [require('./package.json').build?.publish ?? []].flat().find((p) => p.provider === 'github');
  return target ? `${target.owner}/${target.repo}` : 'newtox/hearth';
})();

function newer(a, b) {
  const parts = (v) => String(v).replace(/^v/, '').split(/[.-]/).map((x) => parseInt(x, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) if ((x[i] ?? 0) !== (y[i] ?? 0)) return (x[i] ?? 0) > (y[i] ?? 0);
  return false;
}

async function flatpakRelease() {
  const res = await net.fetch(`https://api.github.com/repos/${repo}/releases/latest`, { headers: { Accept: 'application/vnd.github+json', 'User-Agent': 'Hearth' } });
  if (!res.ok) throw new Error(String(res.status));
  const data = await res.json();
  const asset = (data.assets ?? []).find((a) => /\.flatpak$/i.test(a.name));
  const version = String(data.tag_name ?? '').replace(/^v/, '');
  return asset && newer(version, app.getVersion()) ? { version, url: asset.browser_download_url, size: asset.size } : null;
}

function host(args) {
  return new Promise((resolve) => {
    const child = spawn('flatpak-spawn', ['--host', ...args], { stdio: 'ignore' });
    child.on('error', () => resolve(false));
    child.on('exit', (code) => resolve(code === 0));
  });
}

function flatpakScope() {
  try {
    return /^app-path=.*\/\.local\/share\/flatpak\//m.test(fs.readFileSync('/.flatpak-info', 'utf8')) ? '--user' : '--system';
  } catch {
    return '--user';
  }
}

async function installFlatpak() {
  const release = await flatpakRelease();
  if (!release) return false;
  const dir = process.env.XDG_CACHE_HOME || app.getPath('userData');
  const file = path.join(dir, 'hearth-update.flatpak');
  const res = await net.fetch(release.url);
  if (!res.ok || !res.body) return false;
  const total = Number(res.headers.get('content-length')) || release.size || 0;
  const out = fs.createWriteStream(file);
  let done = 0;
  for await (const chunk of res.body) {
    out.write(chunk);
    done += chunk.length;
    if (total) win?.webContents.send('update-progress', (done * 100) / total);
  }
  await new Promise((resolve, reject) => out.end((err) => (err ? reject(err) : resolve())));
  const ok = await host(['flatpak', 'install', flatpakScope(), '-y', '--noninteractive', '--reinstall', '--bundle', file]);
  fs.rm(file, { force: true }, () => {});
  if (!ok) return false;
  spawn('flatpak-spawn', ['--host', 'sh', '-c', `sleep 2; flatpak run ${process.env.FLATPAK_ID}`], { detached: true, stdio: 'ignore' }).unref();
  setImmediate(() => app.quit());
  return true;
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
  ipcMain.handle('app-version', () => app.getVersion());
  ipcMain.handle('update-check', async () => {
    if (flatpak && app.isPackaged) {
      try {
        const release = await flatpakRelease();
        return { supported: true, version: release?.version ?? null };
      } catch {
        return { supported: false };
      }
    }
    const u = loadUpdater();
    if (!u) return { supported: false };
    try {
      const result = await u.checkForUpdates();
      return { supported: true, version: result?.isUpdateAvailable ? result.updateInfo.version : null };
    } catch {
      return { supported: false };
    }
  });
  ipcMain.handle('update-install', async () => {
    if (flatpak && app.isPackaged) return installFlatpak().catch(() => false);
    const u = loadUpdater();
    if (!u) return false;
    try {
      await u.checkForUpdates();
      await u.downloadUpdate();
      setImmediate(() => u.quitAndInstall(false, true));
      return true;
    } catch {
      return false;
    }
  });
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
