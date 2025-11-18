const MONACO_CDN = 'https://cdn.jsdelivr.net/npm/monaco-editor@0.54.0/min';

window.MonacoEnvironment = {
  getWorkerUrl() {
    const proxy = `
      self.MonacoEnvironment = { baseUrl: '${MONACO_CDN}/' };
      importScripts('${MONACO_CDN}/vs/base/worker/workerMain.js');
    `;
    return `data:text/javascript;charset=utf-8,${encodeURIComponent(proxy)}`;
  },
};

window.require = { paths: { vs: `${MONACO_CDN}/vs` } };
window.MELBI_MONACO_CDN = MONACO_CDN;
