(() => {
  if (!window.__DSH_NO_CONTEXT_MENU__) {
    window.__DSH_NO_CONTEXT_MENU__ = true;
    const block = (event) => {
      event.preventDefault();
      event.stopImmediatePropagation();
      return false;
    };
    const onAux = (event) => {
      if (event.button === 2) block(event);
    };
    window.addEventListener('contextmenu', block, true);
    document.addEventListener('contextmenu', block, true);
    window.addEventListener('auxclick', onAux, true);
    document.addEventListener('auxclick', onAux, true);
    window.addEventListener('mouseup', onAux, true);
  }
  const invoke = (cmd, args) => {
    const internals = window.__TAURI_INTERNALS__;
    if (internals && typeof internals.invoke === 'function') {
      return internals.invoke(cmd, args);
    }
    return Promise.resolve();
  };

  const transparent = (color) => {
    if (!color || color === 'transparent') return true;
    const rgba = color.match(/rgba\(\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*,\s*([\d.]+)\s*\)/i);
    return Boolean(rgba && Number(rgba[4]) === 0);
  };

  const pick = () => {
    const nodes = [];
    const probe = document.elementFromPoint(Math.floor(window.innerWidth / 2), 8);
    if (probe) nodes.push(probe);
    if (document.body) nodes.push(document.body);
    nodes.push(document.documentElement);
    let bg = '';
    let fg = '';
    for (const node of nodes) {
      const cs = getComputedStyle(node);
      if (!fg) fg = cs.color;
      if (!transparent(cs.backgroundColor)) {
        bg = cs.backgroundColor;
        break;
      }
    }
    const root = getComputedStyle(document.documentElement);
    const cssBg =
      root.getPropertyValue('--bg').trim() ||
      root.getPropertyValue('--background').trim() ||
      root.getPropertyValue('--color-background').trim();
    if (!bg && cssBg) bg = cssBg;
    if (!bg) bg = 'rgb(11, 16, 13)';
    if (!fg) fg = 'rgb(231, 242, 228)';
    invoke('report_theme', { bg, fg });
  };

  const start = () => {
    pick();
    const observer = new MutationObserver(pick);
    observer.observe(document.documentElement, {
      attributes: true,
      attributeFilter: ['class', 'style', 'data-theme', 'data-color-mode'],
    });
    if (document.body) {
      observer.observe(document.body, { attributes: true, attributeFilter: ['class', 'style'] });
    }
    setInterval(pick, 2000);
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    if (typeof media.addEventListener === 'function') media.addEventListener('change', pick);
  };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', start);
  else start();
})();
