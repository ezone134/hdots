import QtQuick
import "."

// Thin Text wrapper: forces NativeRendering (hinted, fontconfig-backed text)
// so UI text stays crisp instead of the default blurry QtRendering path.
// Swap `Text {` for `Txt {` to opt in; Text.* enums still resolve via QtQuick.
Text {
    renderType: Text.NativeRendering
}