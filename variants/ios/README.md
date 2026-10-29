# TEZAUR for iPhone / iPad

A WKWebView shell around `tezaur.html` (same page as the Android app). Needs a Mac:

    brew install xcodegen && xcodegen && open Tezaur.xcodeproj    # run on a simulator, or sign with your Apple ID for a device

CI builds it unsigned for the simulator (`ios-simulator` job in `release.yml`). **Not compiled or run yet.**
The model server must be on the same network with CORS enabled (page origin is `file://`). Without a Mac, install the PWA
(`pwa/`) from Safari → Share → Add to Home Screen.
