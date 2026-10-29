// Opens (or focuses) the app in a tab. Extension pages with host_permissions can call a local
// Ollama / Jan server without the server having to enable CORS.
const api = typeof browser !== 'undefined' ? browser : chrome;

api.action.onClicked.addListener(async () => {
  const url = api.runtime.getURL('app/index.html');
  const tabs = await api.tabs.query({ url });
  if (tabs.length) { api.tabs.update(tabs[0].id, { active: true }); api.windows.update(tabs[0].windowId, { focused: true }); }
  else api.tabs.create({ url });
});

// Select text on any page > right-click > "Ask TEZAUR": opens the side panel (Chrome/Edge) or sidebar (Firefox) with the question filled in.
api.runtime.onInstalled.addListener(() => api.contextMenus.create({ id: 'tezaur-ask', title: 'Ask TEZAUR: \u201c%s\u201d', contexts: ['selection'] }));
api.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId !== 'tezaur-ask' || !info.selectionText) return;
  const page = 'app/index.html?q=' + encodeURIComponent(info.selectionText.slice(0, 2000));
  if (api.sidePanel) {
    await api.sidePanel.setOptions({ tabId: tab.id, path: page, enabled: true });
    await api.sidePanel.open({ tabId: tab.id });
  } else if (api.sidebarAction) {
    await api.sidebarAction.setPanel({ panel: api.runtime.getURL(page) });
    await api.sidebarAction.open();
  } else api.tabs.create({ url: api.runtime.getURL(page) });
});
