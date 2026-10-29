# TEZAUR with Capacitor (one project for Android and iOS)

    npm install
    npm run android     # generates android/ ; then:  cd android && ./gradlew assembleDebug   (or open in Android Studio)
    npm run ios         # generates ios/ (needs a Mac); open ios/App/App.xcworkspace in Xcode

Alternative to the hand-written WebView shells in `android/` and `variants/ios/`: same web app, standard plugin ecosystem
(file picker, share sheet, filesystem) if you later want native features. `cleartext: true` lets the page call a model
server on the local network over http; that server must still allow CORS (page origin is https://localhost).
Generated platform folders are git-ignored and rebuilt by CI. Project generation was tested; the native builds were not.
