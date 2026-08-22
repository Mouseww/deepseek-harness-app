(() => {
  if (window.__DSH_NO_CONTEXT_MENU__) return;
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
})();
