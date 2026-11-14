// ClockScreen — preload bridge. The renderer only sees this small API.
'use strict';

const { contextBridge, ipcRenderer } = require('electron');

contextBridge.exposeInMainWorld('clockAPI', {
  getState: () => ipcRenderer.invoke('state:get'),
  onState: (cb) => {
    const h = (_event, s) => cb(s);
    ipcRenderer.on('state', h);
    return () => ipcRenderer.removeListener('state', h);
  },
  dragStart: () => ipcRenderer.send('drag:start'),
  dragEnd: () => ipcRenderer.send('drag:end'),
  scaleBy: (dir) => ipcRenderer.send('scale:step', dir),
  setOpacity: (val) => ipcRenderer.send('opacity:set', val),
  openMenu: () => ipcRenderer.send('menu:open'),
});
