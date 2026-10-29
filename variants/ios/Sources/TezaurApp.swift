import SwiftUI
import WebKit

@main
struct TezaurApp: App {
    var body: some Scene { WindowGroup { WebScreen().ignoresSafeArea(.container, edges: .bottom) } }
}

struct WebScreen: UIViewRepresentable {
    func makeCoordinator() -> Bridge { Bridge() }

    func makeUIView(context: Context) -> WKWebView {
        let cfg = WKWebViewConfiguration()
        cfg.userContentController.add(context.coordinator, name: "save")
        // Same hook the Android build uses: the page calls AndroidBridge.save(name, mime, text).
        let shim = """
        window.AndroidBridge = { save: function(n, m, t) {
          window.webkit.messageHandlers.save.postMessage({name: n, mime: m, text: t}); return 'ok'; } };
        """
        cfg.userContentController.addUserScript(WKUserScript(source: shim, injectionTime: .atDocumentStart, forMainFrameOnly: true))
        let web = WKWebView(frame: .zero, configuration: cfg)
        context.coordinator.web = web
        web.uiDelegate = context.coordinator
        if let url = Bundle.main.url(forResource: "tezaur", withExtension: "html") {
            web.loadFileURL(url, allowingReadAccessTo: url.deletingLastPathComponent())
        }
        return web
    }
    func updateUIView(_ web: WKWebView, context: Context) {}
}

final class Bridge: NSObject, WKScriptMessageHandler, WKUIDelegate {
    weak var web: WKWebView?

    // Saves Markdown / Anki .tsv through the iOS share sheet ("Save to Files").
    func userContentController(_ c: WKUserContentController, didReceive m: WKScriptMessage) {
        guard let d = m.body as? [String: String], let name = d["name"], let text = d["text"] else { return }
        let url = FileManager.default.temporaryDirectory.appendingPathComponent(name)
        try? text.data(using: .utf8)?.write(to: url)
        let vc = UIActivityViewController(activityItems: [url], applicationActivities: nil)
        UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.keyWindow?.rootViewController }.first?
            .present(vc, animated: true)
    }

    // alert()/confirm() from the page.
    func webView(_ w: WKWebView, runJavaScriptAlertPanelWithMessage message: String, initiatedByFrame f: WKFrameInfo, completionHandler: @escaping () -> Void) {
        let a = UIAlertController(title: nil, message: message, preferredStyle: .alert)
        a.addAction(UIAlertAction(title: "OK", style: .default) { _ in completionHandler() })
        UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.keyWindow?.rootViewController }.first?.present(a, animated: true)
    }
    func webView(_ w: WKWebView, runJavaScriptConfirmPanelWithMessage message: String, initiatedByFrame f: WKFrameInfo, completionHandler: @escaping (Bool) -> Void) {
        let a = UIAlertController(title: nil, message: message, preferredStyle: .alert)
        a.addAction(UIAlertAction(title: "OK", style: .default) { _ in completionHandler(true) })
        a.addAction(UIAlertAction(title: "Cancel", style: .cancel) { _ in completionHandler(false) })
        UIApplication.shared.connectedScenes.compactMap { ($0 as? UIWindowScene)?.keyWindow?.rootViewController }.first?.present(a, animated: true)
    }
}
