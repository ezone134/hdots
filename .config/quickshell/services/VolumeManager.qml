pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Services.Pipewire
import "."
Scope {
    id: root
    property real volumeValue: 0.5
    property bool micMuted: false
    property real micVolume: 0.5
    // Set once the initial mic read completes; used by StateController to
    // skip the startup baseline read when deciding whether to toast.
    property bool micInitialized: false
    // Sink mute state. Drives the volume mute/unmute OSD toasts (volumeWatch)
    // and the pill/control-center volume icons.
    property bool volumeMuted: false
    // Set once the initial sink read completes; gates volumeWatch so a
    // device that boots muted doesn't toast at startup (mirrors mic).
    property bool volumeInitialized: false
    // Max device volume/mic levels, read from the dotfiles' state files
    // ($states/max_vol, $states/max_mic — hundredths of the wpctl
    // fraction, e.g. 500 = 500%). UI values (0..1) map to 0..max/100
    // device fraction, so read-backs and writes both scale by these.
    property real maxVolume: 100
    property real maxMic: 100

    // The default sink/source nodes are bound through PwObjectTracker so
    // their `audio` volume/muted become valid and live. Everything the shell
    // needs flows natively from PipeWire here: the CLI (audio_main → wpctl)
    // writes the device and the node emits onVolumeChanged/onMutedChanged,
    // which land in volumeValue/volumeMuted and drive the OSDs with zero IPC
    // and zero process spawning.
    PwObjectTracker {
        objects: [ Pipewire.defaultAudioSink, Pipewire.defaultAudioSource ]
    }

    // max_vol=$(<$states/max_vol); max_mic=$(<$states/max_mic)
    function readMaxLevels() {
        SystemSettingsManager.readBatch([
            SystemSettingsManager.states + "/max_vol",
            SystemSettingsManager.states + "/max_mic"
        ], r => {
            const mv = parseInt((r[0] || "").trim(), 10)
            const mm = parseInt((r[1] || "").trim(), 10)
            if (isFinite(mv) && mv > 0)
                root.maxVolume = mv
            if (isFinite(mm) && mm > 0)
                root.maxMic = mm
            // Re-run the syncs so the UI normalizes against the real maxes.
            root.syncSink()
            root.syncSource()
        })
    }

    Component.onCompleted: {
        root.readMaxLevels()
        root.syncSink()
        root.syncSource()
    }

    // Copy the live sink node state into the UI properties. The audio node's
    // `volume` is a device fraction (0..max/100), so the UI value rescales by
    // maxVolume (e.g. UI 1.0 = 500% device when max_vol=500).
    function syncSink() {
        const a = Pipewire.defaultAudioSink?.audio
        if (!a)
            return
        // volumeValue/volumeMuted are assigned BEFORE volumeInitialized so
        // StateController's watches still see "uninitialized" during the first
        // sync and a sink that boots muted doesn't toast at startup.
        root.volumeValue = Math.max(0, Math.min(1, a.volume * 100 / root.maxVolume))
        root.volumeMuted = a.muted
        root.volumeInitialized = true
    }

    function syncSource() {
        const a = Pipewire.defaultAudioSource?.audio
        if (!a)
            return
        root.micVolume = Math.max(0, Math.min(1, a.volume * 100 / root.maxMic))
        root.micMuted = a.muted
        root.micInitialized = true
    }

    // Live updates from the bound nodes (external/CLI changes included).
    Connections {
        target: Pipewire.defaultAudioSink?.audio
        function onVolumeChanged() { root.syncSink() }
        function onMutedChanged() { root.syncSink() }
    }

    Connections {
        target: Pipewire.defaultAudioSource?.audio
        function onVolumeChanged() { root.syncSource() }
        function onMutedChanged() { root.syncSource() }
    }

    // Follow default-node switches (e.g. headphones unplugged): the audio
    // targets above re-attach automatically, and the explicit syncs cover the
    // brief window where the node is null while PipeWire picks the new default.
    Connections {
        target: Pipewire
        function onDefaultAudioSinkChanged() { root.syncSink() }
        function onDefaultAudioSourceChanged() { root.syncSource() }
    }

    // Writes go straight to the bound node — settable volume/muted propagate
    // to the device via PipeWire, so no wpctl process is spawned. `live` is
    // kept for API compatibility (drag throttling was only needed to rate-limit
    // process spawns; native writes are cheap and stutter-free).
    function setVolume(value, live) {
        StateController.resetHideTimer()
        root.volumeValue = Math.max(0, Math.min(1, value))
        const a = Pipewire.defaultAudioSink?.audio
        if (a)
            a.volume = root.volumeValue * root.maxVolume / 100
    }

    function setMicVolume(value, live) {
        StateController.resetHideTimer()
        root.micVolume = Math.max(0, Math.min(1, value))
        const a = Pipewire.defaultAudioSource?.audio
        if (a)
            a.volume = root.micVolume * root.maxMic / 100
    }

    // Explicit mic mute/unmute (in-process, from the audio panel/control
    // center). Mute state settles reactively via the node's onMutedChanged,
    // which StateController's micWatch turns into the OSD toast.
    function setMicMute(muted) {
        const a = Pipewire.defaultAudioSource?.audio
        if (a)
            a.muted = !!muted
    }

    function setVolumeMute(muted) {
        const a = Pipewire.defaultAudioSink?.audio
        if (a)
            a.muted = !!muted
    }

    function toggleMic() {
        const a = Pipewire.defaultAudioSource?.audio
        if (a)
            a.muted = !a.muted
    }
}
