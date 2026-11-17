const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('hearthDesktop', {
  platform: process.platform,
  minimize: () => ipcRenderer.invoke('window', 'minimize'),
  toggleMaximize: () => ipcRenderer.invoke('window', 'toggle-maximize'),
  close: () => ipcRenderer.invoke('window', 'close'),
  isMaximized: () => ipcRenderer.invoke('window', 'state'),
  onWindowState: (fn) => ipcRenderer.on('window-state', (_e, maximized) => fn(maximized)),
  openExternal: (url) => ipcRenderer.invoke('open-external', url),
  audioShareAvailable: () => ipcRenderer.invoke('audio-share-available'),
  audioShare: (on) => ipcRenderer.invoke('audio-share', on),
});
