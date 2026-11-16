const { app, BrowserWindow, session } = require('electron');
const http = require('http');
const fs = require('fs');
const path = require('path');

const root = path.join(__dirname, 'dist');
const types = { '.html': 'text/html', '.js': 'text/javascript', '.css': 'text/css', '.wasm': 'application/wasm',
  '.json': 'application/json', '.woff2': 'font/woff2', '.woff': 'font/woff', '.png': 'image/png',
  '.svg': 'image/svg+xml', '.map': 'application/json', '.txt': 'text/plain' };

// local server: AudioWorklet + wasm need http origin, file:// fails
const server = http.createServer((req, res) => {
  let p = decodeURIComponent(req.url.split('?')[0]);
  if (p.endsWith('/')) p += 'index.html';
  const f = path.join(root, p);
  if (!f.startsWith(root) || !fs.existsSync(f) || fs.statSync(f).isDirectory()) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { 'Content-Type': types[path.extname(f)] || 'application/octet-stream' });
  fs.createReadStream(f).pipe(res);
});

app.whenReady().then(() => {
  session.defaultSession.setPermissionRequestHandler((_, perm, cb) => cb(['media', 'midi', 'midiSysex'].includes(perm)));
  server.listen(0, '127.0.0.1', () => {
    const win = new BrowserWindow({ width: 1400, height: 900, autoHideMenuBar: true, title: 'CHIMERA' });
    win.loadURL(`http://127.0.0.1:${server.address().port}/`);
  });
});
app.on('window-all-closed', () => app.quit());
