const fs = require('fs');
const { packager } = require('@electron/packager');
fs.cpSync('../dist', 'dist', { recursive: true });
packager({ dir: '.', out: 'out', name: 'CHIMERA', platform: 'win32', arch: 'x64', overwrite: true,
  ignore: [/^\/out/, /^\/pack\.js/, /^\/node_modules\/(?!$)/] })
  .then(p => console.log('built', p));
