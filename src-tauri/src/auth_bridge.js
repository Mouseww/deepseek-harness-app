(() => {
  if (window.__DSH_AUTH_RECOVERY__) return;
  window.__DSH_AUTH_RECOVERY__ = true;

  const nativeFetch = window.fetch.bind(window);
  let redirecting = false;
  let probing = false;

  const onLoginPage = () => window.location.pathname === '/login';

  const requestUrl = (input) => {
    try {
      if (typeof input === 'string' || input instanceof URL) {
        return new URL(input, window.location.origin);
      }
      if (input && typeof input.url === 'string') {
        return new URL(input.url, window.location.origin);
      }
    } catch {
      // An invalid or unsupported request cannot be a same-origin DSH call.
    }
    return null;
  };

  const sameOrigin = (input) => {
    const url = requestUrl(input);
    return url !== null && url.origin === window.location.origin;
  };

  const redirectToLogin = () => {
    if (redirecting || onLoginPage()) return;
    redirecting = true;
    window.location.replace('/login');
  };

  const failedToFetch = (reason) => {
    const message = reason instanceof Error ? reason.message : String(reason ?? '');
    return /failed to fetch|dynamically imported module|importing a module script/i.test(message);
  };

  const probeAuthentication = async () => {
    if (probing || redirecting || onLoginPage()) return;
    probing = true;
    try {
      const response = await nativeFetch('/api/agentPreset.list', {
        method: 'POST',
        credentials: 'same-origin',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({
          type: 'client-request',
          rpcId: 'dsh-desktop-auth-probe',
          method: 'agentPreset.list',
          payload: {},
        }),
      });
      if (response.status === 401) redirectToLogin();
    } catch {
      // A stopped Host is handled by the shell supervisor; do not redirect a
      // genuine network outage to the login page.
    } finally {
      probing = false;
    }
  };

  window.fetch = async (...args) => {
    const input = args[0];
    try {
      const response = await nativeFetch(...args);
      if (response.status === 401 && sameOrigin(input)) redirectToLogin();
      return response;
    } catch (error) {
      if (sameOrigin(input) && failedToFetch(error)) void probeAuthentication();
      throw error;
    }
  };

  window.addEventListener('unhandledrejection', (event) => {
    if (failedToFetch(event.reason)) void probeAuthentication();
  });

  window.addEventListener('error', (event) => {
    const target = event.target;
    const source = target && (target.src || target.href);
    if (typeof source === 'string' && sameOrigin(source)) void probeAuthentication();
  }, true);
})();
